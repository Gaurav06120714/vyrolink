#![allow(clippy::indexing_slicing)] // simulation indices are generated in-range by construction
//! Discrete-event delay-tolerant network simulator.
//!
//! Every node runs the production [`vl_routing::LinkSession`] engine over a
//! [`MemStore`]; encounters exchange real, wire-encoded link messages under a
//! byte budget. What is simulated: mobility (who meets whom, when, for how
//! long), link throughput, participation (iOS-like best-effort nodes),
//! failures and black-hole relays. Results are not Bluetooth measurements.

use std::collections::HashMap;
use vl_crypto::ack_commitment;
use vl_proto::{
    DEFAULT_HOP_LIMIT, Envelope, EnvelopeKind, FLAG_VACCINE, Header, LinkMessage, VaccineItem,
};
use vl_routing::{Algorithm, Event, LinkSession, MemStore, Policy, Store, Stored, maintain};

/// SplitMix64: deterministic, seedable, good enough for simulation (not cryptographic).
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next_u64() % u64::try_from(n.max(1)).unwrap_or(1)).unwrap_or(0)
    }

    /// Exponential with the given mean.
    pub fn exp(&mut self, mean: f64) -> f64 {
        -mean * (1.0 - self.unit()).ln()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Mobility {
    /// Every pair meets as a Poisson process with this rate (encounters per hour).
    Uniform { pair_rate_per_h: f64 },
    /// Nodes split into equal groups; pairs in the same group meet more often.
    Community {
        groups: usize,
        intra_per_h: f64,
        inter_per_h: f64,
    },
}

#[derive(Debug, Clone)]
pub struct Scenario {
    pub name: &'static str,
    pub nodes: usize,
    pub hours: f64,
    pub mobility: Mobility,
    /// Mean encounter duration (s), exponentially distributed.
    pub encounter_mean_s: f64,
    /// Fixed connection + handshake cost per encounter (s).
    pub setup_s: f64,
    /// Assumed application goodput while connected (bytes/s). An assumption, not a measurement.
    pub goodput_bps: f64,
    pub messages: usize,
    pub lifetime_min: u32,
    /// Share of nodes that participate only best-effort (iPhone-like background).
    pub best_effort_share: f64,
    /// Probability a best-effort node manages a session in a given encounter.
    pub best_effort_participation: f64,
    /// Share of nodes that accept custody and silently drop everything.
    pub black_hole_share: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Metrics {
    pub created: usize,
    pub delivered: usize,
    pub acknowledged: usize,
    pub latency_p50_min: f64,
    pub latency_p90_min: f64,
    pub ack_latency_p50_min: f64,
    /// Envelope transfers (data + receipts) per delivered message.
    pub transfers_per_delivery: f64,
    pub peak_store_bytes: usize,
    pub encounters: usize,
    pub sessions: usize,
    /// Link bytes sent, by message type (framing overhead included).
    pub bytes_summary: usize,
    pub bytes_vaccine: usize,
    pub bytes_envelope: usize,
    pub bytes_other: usize,
    /// SUMMARY entries received, and those the receiver already held or had seen.
    pub summary_entries: usize,
    pub summary_entries_redundant: usize,
    /// SUMMARY messages sent, and those caused by a mid-session refresh.
    pub summaries: usize,
    pub summaries_refresh: usize,
}

impl Metrics {
    pub fn summary_share(&self) -> f64 {
        let total =
            self.bytes_summary + self.bytes_vaccine + self.bytes_envelope + self.bytes_other;
        ratio(self.bytes_summary, total)
    }
}

/// Receipt lifetime policy used by the simulator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptLifetime {
    /// Receipt expires with the data envelope (the simulator's behaviour before 2026-10-15).
    WithData,
    /// Fresh `lifetime_min` from creation, as `vl-core` does in production.
    Fresh,
}

impl Metrics {
    pub fn delivery_ratio(&self) -> f64 {
        ratio(self.delivered, self.created)
    }

    pub fn ack_ratio(&self) -> f64 {
        ratio(self.acknowledged, self.created)
    }
}

fn ratio(a: usize, b: usize) -> f64 {
    if b == 0 { 0.0 } else { a as f64 / b as f64 }
}

fn percentile(v: &mut [f64], p: f64) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let idx = ((v.len() - 1) as f64 * p).round() as usize;
    v.get(idx).copied().unwrap_or(f64::NAN)
}

fn tag_of(node: usize) -> [u8; 16] {
    let mut t = [0u8; 16];
    t[..8].copy_from_slice(&(node as u64).to_be_bytes());
    t[8] = 0x7A;
    t
}

fn id_of(n: u64, kind: u8) -> [u8; 16] {
    let mut id = [0u8; 16];
    id[..8].copy_from_slice(&n.to_be_bytes());
    id[8] = kind;
    id
}

struct Node {
    store: MemStore,
    policy: Policy,
    best_effort: bool,
    black_hole: bool,
}

struct Pending {
    at_s: f64,
    from: usize,
    to: usize,
    seq: u64,
}

fn envelope(
    kind: EnvelopeKind,
    id: [u8; 16],
    tag: [u8; 16],
    secret: Option<[u8; 16]>,
    expiry_min: u32,
    budget: u8,
) -> Envelope {
    Envelope {
        header: Header {
            kind,
            flags: if secret.is_some() { FLAG_VACCINE } else { 0 },
            hop_count: 0,
            hop_limit: DEFAULT_HOP_LIMIT,
            copy_budget: budget,
            envelope_id: id,
            recipient_tag: tag,
            expiry_min,
            ack_commitment: secret.map(|s| ack_commitment(&s)).unwrap_or([0; 16]),
            body_len: 256,
        },
        body: vec![0; 256],
    }
}

fn encounters(s: &Scenario, rng: &mut Rng) -> Vec<(f64, usize, usize, f64)> {
    let horizon = s.hours * 3600.0;
    let group_size = match s.mobility {
        Mobility::Community { groups, .. } => s.nodes.div_ceil(groups.max(1)),
        Mobility::Uniform { .. } => s.nodes,
    };
    let mut out = Vec::new();
    for a in 0..s.nodes {
        for b in (a + 1)..s.nodes {
            let rate = match s.mobility {
                Mobility::Uniform { pair_rate_per_h } => pair_rate_per_h,
                Mobility::Community {
                    intra_per_h,
                    inter_per_h,
                    ..
                } => {
                    if a / group_size == b / group_size {
                        intra_per_h
                    } else {
                        inter_per_h
                    }
                }
            };
            if rate <= 0.0 {
                continue;
            }
            let mut t = rng.exp(3600.0 / rate);
            while t < horizon {
                out.push((t, a, b, rng.exp(s.encounter_mean_s)));
                t += rng.exp(3600.0 / rate);
            }
        }
    }
    out.sort_by(|x, y| x.0.total_cmp(&y.0));
    out
}

/// Runs one scenario with one algorithm and seed.
pub fn run(s: &Scenario, algorithm: Algorithm, seed: u64) -> Metrics {
    run_with(s, algorithm, seed, ReceiptLifetime::Fresh)
}

/// Like [`run`] with an explicit receipt-lifetime model (for before/after comparisons).
pub fn run_with(
    s: &Scenario,
    algorithm: Algorithm,
    seed: u64,
    receipts: ReceiptLifetime,
) -> Metrics {
    run_variant(s, algorithm, seed, receipts, |_| {})
}

/// Like [`run_with`], letting an experiment adjust the routing policy of every node.
pub fn run_variant(
    s: &Scenario,
    algorithm: Algorithm,
    seed: u64,
    receipts: ReceiptLifetime,
    tweak: impl Fn(&mut Policy),
) -> Metrics {
    let mut rng = Rng::new(seed);
    let mut policy = Policy::default_spray();
    policy.algorithm = algorithm;
    policy.relay_willing = algorithm != Algorithm::Direct;
    policy.default_lifetime_min = s.lifetime_min;
    tweak(&mut policy);
    let mut nodes: Vec<Node> = (0..s.nodes)
        .map(|i| Node {
            store: MemStore::with_tags([tag_of(i)]),
            policy,
            best_effort: rng.unit() < s.best_effort_share,
            black_hole: rng.unit() < s.black_hole_share,
        })
        .collect();
    let horizon_s = s.hours * 3600.0;
    let mut pending: Vec<Pending> = (0..s.messages)
        .map(|i| {
            let from = rng.below(s.nodes);
            let mut to = rng.below(s.nodes - 1);
            if to >= from {
                to += 1;
            }
            Pending {
                at_s: rng.unit() * horizon_s * 0.5,
                from,
                to,
                seq: i as u64,
            }
        })
        .collect();
    pending.sort_by(|a, b| b.at_s.total_cmp(&a.at_s));

    let mut created_at: HashMap<[u8; 16], f64> = HashMap::new();
    let mut secret_of: HashMap<[u8; 16], [u8; 16]> = HashMap::new();
    let mut sender_of: HashMap<[u8; 16], usize> = HashMap::new();
    let mut delivered_at: HashMap<[u8; 16], f64> = HashMap::new();
    let mut acked_at: HashMap<[u8; 16], f64> = HashMap::new();
    let mut receipt_for: HashMap<[u8; 16], [u8; 16]> = HashMap::new();
    let mut m = Metrics {
        created: s.messages,
        ..Metrics::default()
    };
    let mut transfers = 0usize;

    for (t, a, b, duration) in encounters(s, &mut rng) {
        while pending.last().is_some_and(|p| p.at_s <= t) {
            let Some(p) = pending.pop() else { break };
            let id = id_of(p.seq, 1);
            let secret = id_of(p.seq, 2);
            let expiry = u32::try_from((p.at_s / 60.0) as u64).unwrap_or(0) + s.lifetime_min;
            let env = envelope(
                EnvelopeKind::Data,
                id,
                tag_of(p.to),
                Some(secret),
                expiry,
                policy.initial_budget(),
            );
            let stored_at_min = u32::try_from((p.at_s / 60.0) as u64).unwrap_or(0);
            let _ = nodes[p.from].store.insert(Stored {
                envelope: env,
                own: true,
                stored_at_min,
                retried: false,
            });
            created_at.insert(id, p.at_s);
            secret_of.insert(id, secret);
            sender_of.insert(id, p.from);
        }
        m.encounters += 1;
        let now_min = u32::try_from((t / 60.0) as u64).unwrap_or(0);
        let participates =
            |n: &Node, rng: &mut Rng| !n.best_effort || rng.unit() < s.best_effort_participation;
        if !participates(&nodes[a], &mut rng) || !participates(&nodes[b], &mut rng) {
            continue;
        }
        m.sessions += 1;
        for i in [a, b] {
            let node = &mut nodes[i];
            let _ = maintain(&mut node.store, &node.policy, now_min);
        }
        let mut budget_bytes = ((duration - s.setup_s).max(0.0) * s.goodput_bps) as i64;
        let (mut sa, mut sb) = (
            LinkSession::new(nodes[a].policy),
            LinkSession::new(nodes[b].policy),
        );
        let mut queue: Vec<(usize, LinkMessage)> = Vec::new();
        queue.extend(
            sa.open(&nodes[a].store)
                .unwrap_or_default()
                .into_iter()
                .map(|m| (b, m)),
        );
        queue.extend(
            sb.open(&nodes[b].store)
                .unwrap_or_default()
                .into_iter()
                .map(|m| (a, m)),
        );
        let mut steps = 0;
        while let Some((to, msg)) = (!queue.is_empty()).then(|| queue.remove(0)) {
            steps += 1;
            let size = msg.encode().map(|v| v.len()).unwrap_or(0) as i64 + 18;
            budget_bytes -= size;
            if budget_bytes < 0 || steps > 10_000 {
                break;
            }
            let counted = usize::try_from(size).unwrap_or(0);
            if let LinkMessage::Summary { entries, .. } = &msg {
                m.summaries += 1;
                let rx = &nodes[to].store;
                m.summary_entries += entries.len();
                m.summary_entries_redundant += entries
                    .iter()
                    .filter(|e| {
                        rx.seen(&e.envelope_id).unwrap_or(false)
                            || rx.get(&e.envelope_id).ok().flatten().is_some()
                    })
                    .count();
            }
            match msg {
                LinkMessage::Summary { .. } => m.bytes_summary += counted,
                LinkMessage::Vaccine(_) => m.bytes_vaccine += counted,
                LinkMessage::Envelope(_) => m.bytes_envelope += counted,
                _ => m.bytes_other += counted,
            }
            if matches!(msg, LinkMessage::Envelope(_)) {
                transfers += 1;
            }
            let (session, node) = if to == a {
                (&mut sa, &mut nodes[a])
            } else {
                (&mut sb, &mut nodes[b])
            };
            let (replies, events) = session.handle(msg, &mut node.store, now_min);
            let from = if to == a { b } else { a };
            queue.extend(replies.into_iter().map(|r| (from, r)));
            let mut refresh = false;
            for e in events {
                if let Event::Inbound(env) = e {
                    let id = env.header.envelope_id;
                    match env.header.kind {
                        EnvelopeKind::Data => {
                            delivered_at.entry(id).or_insert(t);
                            let secret = secret_of.get(&id).copied().unwrap_or([0; 16]);
                            let _ = node.store.add_vaccine(
                                VaccineItem {
                                    ack_secret: secret,
                                    expiry_min: env.header.expiry_min,
                                },
                                env.header.ack_commitment,
                            );
                            let rid = {
                                let mut r = id;
                                r[8] = 3;
                                r
                            };
                            receipt_for.insert(rid, id);
                            let sender = sender_of.get(&id).copied().unwrap_or(0);
                            let receipt_expiry = match receipts {
                                ReceiptLifetime::WithData => env.header.expiry_min,
                                ReceiptLifetime::Fresh => now_min + s.lifetime_min,
                            };
                            let receipt = envelope(
                                EnvelopeKind::Receipt,
                                rid,
                                tag_of(sender),
                                None,
                                receipt_expiry,
                                policy.initial_budget(),
                            );
                            let _ = node.store.insert(Stored {
                                envelope: receipt,
                                own: true,
                                stored_at_min: now_min,
                                retried: false,
                            });
                            refresh = true;
                        }
                        EnvelopeKind::Receipt => {
                            if let Some(orig) = receipt_for.get(&id) {
                                acked_at.entry(*orig).or_insert(t);
                            }
                        }
                    }
                } else if let Event::OwnDelivered { envelope_id, .. } = e {
                    acked_at.entry(envelope_id).or_insert(t);
                }
            }
            if refresh {
                let s_ref = if to == a { &mut sa } else { &mut sb };
                m.summaries_refresh += 1;
                queue.extend(
                    s_ref
                        .refresh(&node.store, now_min)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|r| (from, r)),
                );
            }
        }
        for i in [a, b] {
            if nodes[i].black_hole {
                let carried: Vec<_> = nodes[i]
                    .store
                    .all()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|st| !st.own)
                    .collect();
                for st in carried {
                    let _ = nodes[i].store.remove(&st.envelope.header.envelope_id);
                }
            }
            let bytes: usize = nodes[i]
                .store
                .all()
                .unwrap_or_default()
                .iter()
                .map(|st| st.envelope.body.len() + vl_proto::HEADER_LEN)
                .sum();
            m.peak_store_bytes = m.peak_store_bytes.max(bytes);
        }
    }

    let mut lat: Vec<f64> = delivered_at
        .iter()
        .filter_map(|(id, t)| created_at.get(id).map(|c| (t - c) / 60.0))
        .collect();
    let mut ack_lat: Vec<f64> = acked_at
        .iter()
        .filter_map(|(id, t)| created_at.get(id).map(|c| (t - c) / 60.0))
        .collect();
    m.delivered = delivered_at.len();
    m.acknowledged = acked_at.len();
    m.latency_p50_min = percentile(&mut lat, 0.5);
    m.latency_p90_min = percentile(&mut lat, 0.9);
    m.ack_latency_p50_min = percentile(&mut ack_lat, 0.5);
    m.transfers_per_delivery = if m.delivered == 0 {
        f64::NAN
    } else {
        transfers as f64 / m.delivered as f64
    };
    m
}

/// The six scenarios of Doc 22 (shared by the report and the experiments).
pub fn standard_scenarios() -> Vec<Scenario> {
    let campus = Scenario {
        name: "campus (40 nodes, 4 communities, 72 h)",
        nodes: 40,
        hours: 72.0,
        mobility: Mobility::Community {
            groups: 4,
            intra_per_h: 0.25,
            inter_per_h: 0.01,
        },
        encounter_mean_s: 120.0,
        setup_s: 3.0,
        goodput_bps: 20_000.0,
        messages: 200,
        lifetime_min: 72 * 60,
        best_effort_share: 0.5,
        best_effort_participation: 0.5,
        black_hole_share: 0.0,
    };
    vec![
        campus.clone(),
        Scenario {
            name: "sparse (30 nodes, uniform, 72 h)",
            nodes: 30,
            mobility: Mobility::Uniform {
                pair_rate_per_h: 0.01,
            },
            ..campus.clone()
        },
        Scenario {
            name: "dense event (100 nodes, 12 h)",
            nodes: 100,
            hours: 12.0,
            mobility: Mobility::Uniform {
                pair_rate_per_h: 0.05,
            },
            encounter_mean_s: 60.0,
            messages: 400,
            lifetime_min: 12 * 60,
            ..campus.clone()
        },
        Scenario {
            name: "campus + 20% black-hole relays",
            black_hole_share: 0.2,
            ..campus.clone()
        },
        Scenario {
            name: "campus, short windows (10 s mean, 5 s setup)",
            encounter_mean_s: 10.0,
            setup_s: 5.0,
            ..campus.clone()
        },
        Scenario {
            name: "campus, all nodes always-on (Android FGS backbone)",
            best_effort_share: 0.0,
            ..campus
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small() -> Scenario {
        Scenario {
            name: "test",
            nodes: 12,
            hours: 24.0,
            mobility: Mobility::Uniform {
                pair_rate_per_h: 0.3,
            },
            encounter_mean_s: 120.0,
            setup_s: 3.0,
            goodput_bps: 20_000.0,
            messages: 30,
            lifetime_min: 24 * 60,
            best_effort_share: 0.0,
            best_effort_participation: 1.0,
            black_hole_share: 0.0,
        }
    }

    #[test]
    fn deterministic_for_a_seed() {
        let s = small();
        let alg = Algorithm::SprayAndWait { initial: 4 };
        assert_eq!(run(&s, alg, 7), run(&s, alg, 7));
    }

    #[test]
    fn relaying_beats_direct_delivery_when_contacts_are_sparse() {
        let s = small();
        let direct = run(&s, Algorithm::Direct, 3);
        let spray = run(&s, Algorithm::SprayAndWait { initial: 4 }, 3);
        assert!(spray.delivered >= direct.delivered);
        assert!(direct.delivered <= direct.created);
        assert!(spray.acknowledged <= spray.delivered + spray.created);
    }

    #[test]
    fn nobody_meets_nothing_delivered() {
        let mut s = small();
        s.mobility = Mobility::Uniform {
            pair_rate_per_h: 0.0,
        };
        let m = run(&s, Algorithm::Epidemic, 1);
        assert_eq!(m.delivered, 0);
        assert_eq!(m.encounters, 0);
    }
}
