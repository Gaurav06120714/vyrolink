use crate::{Algorithm, Policy, Store, StoreResult, Stored};
use std::collections::{BTreeMap, HashMap, HashSet};
use vl_crypto::ack_commitment;
use vl_proto::{
    Admission, CustodyItem, CustodyStatus, Envelope, HEADER_LEN, Hello, LINK_VERSION, LinkMessage,
    MAX_FRAME_LEN, MAX_ITEMS, RelayClass, RequestItem, RequestRole, SummaryEntry, VaccineItem,
};

pub const BYE_INCOMPATIBLE: u8 = 1;
pub const BYE_STORE_FAILURE: u8 = 2;

/// Effects the application must act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// An envelope whose tag is ours. The app opens it (vl-crypto) and, on success,
    /// stores the vaccine and queues a receipt.
    Inbound(Envelope),
    /// A neighbour took custody of our own envelope: ACCEPTED_BY_RELAY, never "Delivered".
    OwnAccepted([u8; 16]),
    /// A vaccine whose secret matches our own envelope's commitment. Only the
    /// recipient (or we) can know that secret, so this proves the recipient
    /// unwrapped the message (ADR-020).
    OwnDelivered {
        envelope_id: [u8; 16],
        ack_secret: [u8; 16],
    },
    /// Our own envelope reached its expiry without delivery proof.
    OwnExpired([u8; 16]),
    PeerIncompatible,
    Closed,
}

/// Replication split for one CARRY transfer: `(copies given, copies kept)`.
pub fn split_budget(algorithm: Algorithm, budget: u8) -> Option<(u8, u8)> {
    match algorithm {
        Algorithm::Direct => None,
        Algorithm::Epidemic => (budget >= 1).then_some((budget, budget)),
        Algorithm::SprayAndWait { .. } => (budget > 1).then(|| (budget / 2, budget - budget / 2)),
    }
}

fn wire_len(e: &Envelope) -> usize {
    HEADER_LEN + e.body.len()
}

/// One neighbour connection. Order-independent: both sides call [`LinkSession::open`]
/// and then feed every received message to [`LinkSession::handle`].
#[derive(Debug)]
pub struct LinkSession {
    policy: Policy,
    peer: Option<Hello>,
    pending_spray: HashMap<[u8; 16], u8>,
    requested_carry: HashSet<[u8; 16]>,
    requested_tags: HashMap<[u8; 16], usize>,
    reserved_bytes: usize,
    accepted_envelopes: usize,
    accepted_bytes: usize,
    offered: HashSet<[u8; 16]>,
    vaccines_sent: HashSet<[u8; 16]>,
    closed: bool,
}

impl LinkSession {
    pub fn new(policy: Policy) -> Self {
        Self {
            policy,
            peer: None,
            pending_spray: HashMap::new(),
            requested_carry: HashSet::new(),
            requested_tags: HashMap::new(),
            reserved_bytes: 0,
            accepted_envelopes: 0,
            accepted_bytes: 0,
            offered: HashSet::new(),
            vaccines_sent: HashSet::new(),
            closed: false,
        }
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    pub fn peer(&self) -> Option<&Hello> {
        self.peer.as_ref()
    }

    fn carries(&self) -> bool {
        self.policy.relay_willing
            && self.policy.relay_class != RelayClass::EndpointOnly
            && self.policy.algorithm != Algorithm::Direct
    }

    pub fn open(&self, store: &dyn Store) -> StoreResult<Vec<LinkMessage>> {
        let free = self
            .policy
            .carry_cap_bytes
            .saturating_sub(store.carried_bytes()?);
        let bucket = (free.saturating_mul(255) / self.policy.carry_cap_bytes.max(1)).min(255);
        Ok(vec![LinkMessage::Hello(Hello {
            link_versions: vec![LINK_VERSION],
            max_frame: MAX_FRAME_LEN as u16,
            relay_class: self.policy.relay_class,
            relay_willing: self.carries(),
            free_capacity_bucket: u8::try_from(bucket).unwrap_or(u8::MAX),
            l2cap_psm: None,
            // The node adds FEATURE_PRESENCE; routing has no optional features.
            features: 0,
        })])
    }

    /// VACCINE and SUMMARY messages for when local state changed mid-session
    /// (e.g. a receipt was just created and the sender is right here). With
    /// `delta_refresh`, only items not yet sent in this session are included,
    /// and nothing is sent when there is nothing new.
    pub fn refresh(&mut self, store: &dyn Store, now_min: u32) -> StoreResult<Vec<LinkMessage>> {
        self.vaccine_and_summary(store, now_min, self.policy.delta_refresh)
    }

    fn vaccine_and_summary(
        &mut self,
        store: &dyn Store,
        now_min: u32,
        delta: bool,
    ) -> StoreResult<Vec<LinkMessage>> {
        let mut out = Vec::new();
        let mut vaccines = store.vaccines(now_min, MAX_ITEMS)?;
        if delta {
            vaccines.retain(|v| !self.vaccines_sent.contains(&ack_commitment(&v.ack_secret)));
        }
        self.vaccines_sent
            .extend(vaccines.iter().map(|v| ack_commitment(&v.ack_secret)));
        if !vaccines.is_empty() {
            out.push(LinkMessage::Vaccine(vaccines));
        }
        let skip = if delta {
            &self.offered
        } else {
            &HashSet::new()
        };
        let entries = offers(store, self.policy.algorithm, now_min, skip)?;
        self.offered.extend(entries.iter().map(|e| e.envelope_id));
        if !delta || !entries.is_empty() {
            out.push(LinkMessage::Summary {
                entries,
                delivered: Vec::new(),
            });
        }
        Ok(out)
    }

    /// Processes one peer message. On a storage error the session closes and a
    /// BYE is returned; every store call is atomic, so no partial transfer is recorded.
    pub fn handle(
        &mut self,
        msg: LinkMessage,
        store: &mut dyn Store,
        now_min: u32,
    ) -> (Vec<LinkMessage>, Vec<Event>) {
        if self.closed {
            return (Vec::new(), Vec::new());
        }
        let mut out = Vec::new();
        let mut events = Vec::new();
        match self.dispatch(msg, store, now_min, &mut out, &mut events) {
            Ok(()) => (out, events),
            Err(_) => {
                self.closed = true;
                events.push(Event::Closed);
                (vec![LinkMessage::Bye(BYE_STORE_FAILURE)], events)
            }
        }
    }

    fn dispatch(
        &mut self,
        msg: LinkMessage,
        store: &mut dyn Store,
        now_min: u32,
        out: &mut Vec<LinkMessage>,
        events: &mut Vec<Event>,
    ) -> StoreResult<()> {
        match msg {
            LinkMessage::Hello(h) => {
                if !h.link_versions.contains(&LINK_VERSION) {
                    self.closed = true;
                    out.push(LinkMessage::Bye(BYE_INCOMPATIBLE));
                    events.push(Event::PeerIncompatible);
                    return Ok(());
                }
                self.peer = Some(h);
                out.extend(self.vaccine_and_summary(store, now_min, false)?);
            }
            LinkMessage::Vaccine(items) => self.on_vaccine(items, store, now_min, events)?,
            LinkMessage::Summary { entries, .. } => {
                let request = self.on_summary(&entries, store, now_min)?;
                if !request.is_empty() {
                    out.push(LinkMessage::Request(request));
                }
            }
            LinkMessage::Request(items) => out.extend(self.on_request(&items, store)?),
            LinkMessage::Envelope(env) => {
                let status = self.on_envelope(env, store, now_min, events)?;
                out.push(LinkMessage::Custody(vec![status]));
            }
            LinkMessage::Custody(items) => self.on_custody(&items, store, events)?,
            LinkMessage::Ping(t) => out.push(LinkMessage::Pong(t)),
            LinkMessage::Pong(_) => {}
            // Contact presence (ADR-029) belongs to the node, not to routing.
            LinkMessage::Presence(_) => {}
            LinkMessage::Bye(_) => {
                self.closed = true;
                events.push(Event::Closed);
            }
        }
        Ok(())
    }

    /// Vaccines are accepted only if they match an envelope we hold, so forged
    /// secrets can neither purge mail nor consume storage.
    fn on_vaccine(
        &mut self,
        items: Vec<VaccineItem>,
        store: &mut dyn Store,
        now_min: u32,
        events: &mut Vec<Event>,
    ) -> StoreResult<()> {
        let horizon = now_min
            .saturating_add(self.policy.max_lifetime_min)
            .saturating_add(self.policy.skew_min);
        for item in items {
            if item.expiry_min <= now_min || item.expiry_min > horizon {
                continue;
            }
            let commitment = ack_commitment(&item.ack_secret);
            if store.has_vaccine(&commitment)? {
                continue;
            }
            let holders = store.vaccine_holders(&commitment)?;
            if holders.is_empty() {
                continue;
            }
            for (envelope_id, _) in holders.iter().filter(|(_, own)| *own) {
                events.push(Event::OwnDelivered {
                    envelope_id: *envelope_id,
                    ack_secret: item.ack_secret,
                });
            }
            store.add_vaccine(item, commitment)?;
        }
        Ok(())
    }

    fn on_summary(
        &mut self,
        entries: &[SummaryEntry],
        store: &dyn Store,
        now_min: u32,
    ) -> StoreResult<Vec<RequestItem>> {
        let mut request = Vec::new();
        let carried_bytes = store.carried_bytes()?;
        for e in entries {
            if request.len() == MAX_ITEMS {
                break;
            }
            let id = e.envelope_id;
            if e.expiry_min <= now_min
                || self.requested_carry.contains(&id)
                || store.seen(&id)?
                || store.get(&id)?.is_some()
            {
                continue;
            }
            if store.is_my_tag(&e.recipient_tag)? {
                request.push(RequestItem {
                    envelope_id: id,
                    role: RequestRole::Deliver,
                });
                continue;
            }
            if !self.carries() || !e.spray {
                continue;
            }
            let size = HEADER_LEN + usize::from(e.body_len);
            let tag_count = store.carried_for_tag(&e.recipient_tag)?
                + self
                    .requested_tags
                    .get(&e.recipient_tag)
                    .copied()
                    .unwrap_or(0);
            let within = self.requested_carry.len() + self.accepted_envelopes
                < self.policy.session_accept_envelopes
                && self.reserved_bytes + self.accepted_bytes + size
                    <= self.policy.session_accept_bytes
                && carried_bytes + self.reserved_bytes + size <= self.policy.carry_cap_bytes
                && tag_count < self.policy.per_tag_cap;
            if within {
                self.requested_carry.insert(id);
                *self.requested_tags.entry(e.recipient_tag).or_insert(0) += 1;
                self.reserved_bytes += size;
                request.push(RequestItem {
                    envelope_id: id,
                    role: RequestRole::Carry,
                });
            }
        }
        Ok(request)
    }

    fn on_request(
        &mut self,
        items: &[RequestItem],
        store: &dyn Store,
    ) -> StoreResult<Vec<LinkMessage>> {
        let mut out = Vec::new();
        for item in items {
            let Some(stored) = store.get(&item.envelope_id)? else {
                continue;
            };
            let mut env = stored.envelope;
            match item.role {
                RequestRole::Deliver => out.push(LinkMessage::Envelope(env)),
                RequestRole::Carry => {
                    if let Some((give, keep)) =
                        split_budget(self.policy.algorithm, env.header.copy_budget)
                    {
                        env.header.copy_budget = give;
                        self.pending_spray.insert(item.envelope_id, keep);
                        out.push(LinkMessage::Envelope(env));
                    }
                }
            }
        }
        Ok(out)
    }

    fn on_envelope(
        &mut self,
        mut env: Envelope,
        store: &mut dyn Store,
        now_min: u32,
        events: &mut Vec<Event>,
    ) -> StoreResult<CustodyItem> {
        let id = env.header.envelope_id;
        let status = |status| CustodyItem {
            envelope_id: id,
            status,
        };
        if env
            .header
            .admission(now_min, self.policy.max_lifetime_min, self.policy.skew_min)
            != Admission::Accept
        {
            return Ok(status(CustodyStatus::RejectedPolicy));
        }
        if store.seen(&id)?
            || store.get(&id)?.is_some()
            || (env.header.vaccine_enabled() && store.has_vaccine(&env.header.ack_commitment)?)
        {
            return Ok(status(CustodyStatus::Duplicate));
        }
        if store.is_my_tag(&env.header.recipient_tag)? {
            store.mark_seen(&id, env.header.expiry_min)?;
            events.push(Event::Inbound(env));
            return Ok(status(CustodyStatus::DeliveredLocal));
        }
        if !self.carries() || !self.requested_carry.remove(&id) {
            return Ok(status(CustodyStatus::RejectedPolicy));
        }
        let size = wire_len(&env);
        self.reserved_bytes = self.reserved_bytes.saturating_sub(size);
        if let Some(n) = self.requested_tags.get_mut(&env.header.recipient_tag) {
            *n = n.saturating_sub(1);
        }
        if store.carried_bytes()? + size > self.policy.carry_cap_bytes
            || store.carried_for_tag(&env.header.recipient_tag)? >= self.policy.per_tag_cap
            || self.accepted_bytes + size > self.policy.session_accept_bytes
        {
            return Ok(status(CustodyStatus::RejectedFull));
        }
        env.on_relay_accept();
        let expiry = env.header.expiry_min;
        store.insert(Stored {
            envelope: env,
            own: false,
            stored_at_min: now_min,
            retried: false,
        })?;
        store.mark_seen(&id, expiry)?;
        self.accepted_envelopes += 1;
        self.accepted_bytes += size;
        Ok(status(CustodyStatus::Accepted))
    }

    fn on_custody(
        &mut self,
        items: &[CustodyItem],
        store: &mut dyn Store,
        events: &mut Vec<Event>,
    ) -> StoreResult<()> {
        for item in items {
            let keep = self.pending_spray.remove(&item.envelope_id);
            let own = store.get(&item.envelope_id)?.is_some_and(|s| s.own);
            match item.status {
                CustodyStatus::Accepted => {
                    if let Some(keep) = keep {
                        store.set_budget(&item.envelope_id, keep)?;
                    }
                    if own {
                        events.push(Event::OwnAccepted(item.envelope_id));
                    }
                }
                CustodyStatus::DeliveredLocal if own => {
                    events.push(Event::OwnAccepted(item.envelope_id))
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// Summary of what we hold, own envelopes first, then round-robin across
/// recipient tags (fairness between conversations), soonest expiry first per tag.
fn offers(
    store: &dyn Store,
    algorithm: Algorithm,
    now_min: u32,
    skip: &HashSet<[u8; 16]>,
) -> StoreResult<Vec<SummaryEntry>> {
    let mut groups: [BTreeMap<[u8; 16], Vec<Stored>>; 2] = [BTreeMap::new(), BTreeMap::new()];
    for s in store.all()?.into_iter().filter(|s| {
        s.envelope.header.expiry_min > now_min && !skip.contains(&s.envelope.header.envelope_id)
    }) {
        if let Some(group) = groups.get_mut(usize::from(!s.own)) {
            group
                .entry(s.envelope.header.recipient_tag)
                .or_default()
                .push(s);
        }
    }
    let mut out = Vec::new();
    for group in groups {
        let mut queues: Vec<std::vec::IntoIter<Stored>> = group
            .into_values()
            .map(|mut v| {
                v.sort_by_key(|s| s.envelope.header.expiry_min);
                v.into_iter()
            })
            .collect();
        while out.len() < MAX_ITEMS && !queues.is_empty() {
            queues.retain_mut(|q| match q.next() {
                Some(s) if out.len() < MAX_ITEMS => {
                    let h = &s.envelope.header;
                    out.push(SummaryEntry {
                        envelope_id: h.envelope_id,
                        recipient_tag: h.recipient_tag,
                        expiry_min: h.expiry_min,
                        body_len: h.body_len,
                        spray: split_budget(algorithm, h.copy_budget).is_some(),
                    });
                    true
                }
                _ => false,
            });
        }
    }
    Ok(out)
}

/// Periodic upkeep: expire records and run the sender's single re-spray (Doc 18 §7.3 step 6).
pub fn maintain(store: &mut dyn Store, policy: &Policy, now_min: u32) -> StoreResult<Vec<Event>> {
    let events: Vec<Event> = store
        .purge_expired(now_min)?
        .into_iter()
        .filter(|s| s.own)
        .map(|s| Event::OwnExpired(s.envelope.header.envelope_id))
        .collect();
    if let Algorithm::SprayAndWait { .. } = policy.algorithm {
        for s in store
            .all()?
            .into_iter()
            .filter(|s| s.own && !s.retried && s.envelope.header.copy_budget <= 1)
        {
            let lifetime = s.envelope.header.expiry_min.saturating_sub(s.stored_at_min);
            let elapsed = now_min.saturating_sub(s.stored_at_min);
            if u64::from(elapsed) * 100
                >= u64::from(lifetime) * u64::from(policy.retry_after_percent)
            {
                store.set_budget(&s.envelope.header.envelope_id, policy.initial_budget())?;
                store.set_retried(&s.envelope.header.envelope_id)?;
            }
        }
    }
    Ok(events)
}
