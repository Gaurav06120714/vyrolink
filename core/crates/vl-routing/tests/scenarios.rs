#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use vl_crypto::ack_commitment;
use vl_proto::*;
use vl_routing::*;

const NOW: u32 = 29_000_000;

fn envelope(id: u8, tag: [u8; 16], budget: u8, secret: [u8; 16]) -> Envelope {
    Envelope {
        header: Header {
            kind: EnvelopeKind::Data,
            flags: FLAG_VACCINE,
            hop_count: 0,
            hop_limit: DEFAULT_HOP_LIMIT,
            copy_budget: budget,
            envelope_id: [id; 16],
            recipient_tag: tag,
            expiry_min: NOW + 72 * 60,
            ack_commitment: ack_commitment(&secret),
            body_len: 256,
        },
        body: vec![0xEE; 256],
    }
}

fn own(store: &mut MemStore, env: Envelope) {
    store
        .insert(Stored {
            envelope: env,
            own: true,
            stored_at_min: NOW,
            retried: false,
        })
        .unwrap();
}

struct Node {
    store: MemStore,
    policy: Policy,
    events: Vec<Event>,
}

impl Node {
    fn new(tags: &[[u8; 16]]) -> Self {
        Self {
            store: MemStore::with_tags(tags.iter().copied()),
            policy: Policy::default_spray(),
            events: Vec::new(),
        }
    }
}

/// Runs one encounter, round-tripping every message through the wire codec.
fn encounter(a: &mut Node, b: &mut Node, now: u32) {
    let mut sa = LinkSession::new(a.policy);
    let mut sb = LinkSession::new(b.policy);
    let mut to_b: Vec<LinkMessage> = sa.open(&a.store).unwrap();
    let mut to_a: Vec<LinkMessage> = sb.open(&b.store).unwrap();
    for _ in 0..32 {
        if to_a.is_empty() && to_b.is_empty() {
            return;
        }
        let mut next_to_a = Vec::new();
        for m in to_b.drain(..) {
            let m = LinkMessage::decode(&m.encode().unwrap()).unwrap();
            let (out, ev) = sb.handle(m, &mut b.store, now);
            next_to_a.extend(out);
            b.events.extend(ev);
        }
        let mut next_to_b = Vec::new();
        for m in to_a.drain(..) {
            let m = LinkMessage::decode(&m.encode().unwrap()).unwrap();
            let (out, ev) = sa.handle(m, &mut a.store, now);
            next_to_b.extend(out);
            a.events.extend(ev);
        }
        to_a = next_to_a;
        to_b = next_to_b;
    }
    panic!("encounter did not quiesce");
}

const C_TAG: [u8; 16] = [0xC; 16];
const SECRET: [u8; 16] = [0x5E; 16];

#[test]
fn alice_bob_charlie_store_carry_forward_with_binary_spray() {
    let (mut alice, mut bob, mut charlie) = (Node::new(&[]), Node::new(&[]), Node::new(&[C_TAG]));
    own(&mut alice.store, envelope(1, C_TAG, 4, SECRET));

    encounter(&mut alice, &mut bob, NOW);
    assert_eq!(
        alice
            .store
            .get(&[1; 16])
            .unwrap()
            .unwrap()
            .envelope
            .header
            .copy_budget,
        2,
        "sender keeps ceil(4/2)"
    );
    let carried = bob.store.get(&[1; 16]).unwrap().unwrap();
    assert!(!carried.own);
    assert_eq!(carried.envelope.header.copy_budget, 2);
    assert_eq!(carried.envelope.header.hop_count, 1);
    assert_eq!(
        alice.events,
        vec![Event::OwnAccepted([1; 16])],
        "relay acceptance is not delivery"
    );

    encounter(&mut bob, &mut charlie, NOW + 60);
    assert!(
        matches!(charlie.events.as_slice(), [Event::Inbound(e)] if e.header.envelope_id == [1; 16])
    );
    assert!(
        bob.store.get(&[1; 16]).unwrap().is_some(),
        "carrier keeps its copy until a vaccine"
    );

    charlie
        .store
        .add_vaccine(
            VaccineItem {
                ack_secret: SECRET,
                expiry_min: NOW + 72 * 60,
            },
            ack_commitment(&SECRET),
        )
        .unwrap();
    encounter(&mut charlie, &mut bob, NOW + 120);
    assert!(
        bob.store.get(&[1; 16]).unwrap().is_none(),
        "vaccine purged the carried copy"
    );

    encounter(&mut bob, &mut alice, NOW + 180);
    assert!(
        alice.events.contains(&Event::OwnDelivered {
            envelope_id: [1; 16],
            ack_secret: SECRET
        }),
        "bob relayed the delivery proof back to alice"
    );
    assert!(alice.store.get(&[1; 16]).unwrap().is_none());
}

#[test]
fn duplicate_paths_deliver_once() {
    let (mut alice, mut bob, mut dave, mut charlie) = (
        Node::new(&[]),
        Node::new(&[]),
        Node::new(&[]),
        Node::new(&[C_TAG]),
    );
    own(&mut alice.store, envelope(2, C_TAG, 4, SECRET));
    encounter(&mut alice, &mut bob, NOW);
    encounter(&mut alice, &mut dave, NOW);
    assert_eq!(
        alice
            .store
            .get(&[2; 16])
            .unwrap()
            .unwrap()
            .envelope
            .header
            .copy_budget,
        1
    );
    assert_eq!(
        dave.store
            .get(&[2; 16])
            .unwrap()
            .unwrap()
            .envelope
            .header
            .copy_budget,
        1
    );
    encounter(&mut bob, &mut charlie, NOW + 10);
    encounter(&mut dave, &mut charlie, NOW + 20);
    let inbound = charlie
        .events
        .iter()
        .filter(|e| matches!(e, Event::Inbound(_)))
        .count();
    assert_eq!(inbound, 1);
}

#[test]
fn wait_phase_carrier_only_delivers_to_recipient() {
    let (mut bob, mut eve) = (Node::new(&[]), Node::new(&[]));
    bob.store
        .insert(Stored {
            envelope: envelope(3, C_TAG, 1, SECRET),
            own: false,
            stored_at_min: NOW,
            retried: false,
        })
        .unwrap();
    encounter(&mut bob, &mut eve, NOW);
    assert!(eve.store.get(&[3; 16]).unwrap().is_none());
}

#[test]
fn unsolicited_envelopes_and_endpoint_only_peers_are_refused() {
    let mut node = Node::new(&[]);
    let mut session = LinkSession::new(node.policy);
    let (out, _) = session.handle(
        LinkMessage::Envelope(envelope(4, C_TAG, 2, SECRET)),
        &mut node.store,
        NOW,
    );
    assert_eq!(
        out,
        vec![LinkMessage::Custody(vec![CustodyItem {
            envelope_id: [4; 16],
            status: CustodyStatus::RejectedPolicy
        }])]
    );

    let (mut alice, mut endpoint) = (Node::new(&[]), Node::new(&[]));
    endpoint.policy.relay_class = RelayClass::EndpointOnly;
    own(&mut alice.store, envelope(5, C_TAG, 4, SECRET));
    encounter(&mut alice, &mut endpoint, NOW);
    assert!(endpoint.store.get(&[5; 16]).unwrap().is_none());
    assert_eq!(
        alice
            .store
            .get(&[5; 16])
            .unwrap()
            .unwrap()
            .envelope
            .header
            .copy_budget,
        4
    );
}

#[test]
fn per_tag_and_capacity_quotas_bound_storage() {
    let (mut flooder, mut relay) = (Node::new(&[]), Node::new(&[]));
    for i in 0..40u8 {
        own(&mut flooder.store, envelope(100 + i, C_TAG, 8, [i; 16]));
    }
    encounter(&mut flooder, &mut relay, NOW);
    assert_eq!(
        relay.store.carried_for_tag(&C_TAG).unwrap(),
        relay.policy.per_tag_cap
    );

    let (mut flooder2, mut relay2) = (Node::new(&[]), Node::new(&[]));
    for i in 0..60u8 {
        own(&mut flooder2.store, envelope(i, [i; 16], 8, [i; 16]));
    }
    encounter(&mut flooder2, &mut relay2, NOW);
    assert!(relay2.store.all().unwrap().len() <= relay2.policy.session_accept_envelopes);
    assert!(relay2.store.carried_bytes().unwrap() <= relay2.policy.session_accept_bytes);
}

#[test]
fn inflated_budgets_are_clamped_and_expired_envelopes_rejected() {
    let (mut liar, mut relay) = (Node::new(&[]), Node::new(&[]));
    own(&mut liar.store, envelope(6, C_TAG, 200, SECRET));
    encounter(&mut liar, &mut relay, NOW);
    assert!(
        relay
            .store
            .get(&[6; 16])
            .unwrap()
            .unwrap()
            .envelope
            .header
            .copy_budget
            <= MAX_COPY_BUDGET
    );

    let mut stale = envelope(7, C_TAG, 4, SECRET);
    stale.header.expiry_min = NOW;
    let mut s = LinkSession::new(relay.policy);
    let (out, _) = s.handle(LinkMessage::Envelope(stale), &mut relay.store, NOW);
    assert_eq!(
        out,
        vec![LinkMessage::Custody(vec![CustodyItem {
            envelope_id: [7; 16],
            status: CustodyStatus::RejectedPolicy
        }])]
    );
}

#[test]
fn forged_vaccines_are_ignored_and_never_stored() {
    let (mut attacker, mut relay) = (Node::new(&[]), Node::new(&[]));
    relay
        .store
        .insert(Stored {
            envelope: envelope(8, C_TAG, 2, SECRET),
            own: false,
            stored_at_min: NOW,
            retried: false,
        })
        .unwrap();
    attacker
        .store
        .add_vaccine(
            VaccineItem {
                ack_secret: [0xBA; 16],
                expiry_min: NOW + 60,
            },
            ack_commitment(&[0xBA; 16]),
        )
        .unwrap();
    let mut s = LinkSession::new(relay.policy);
    s.handle(
        LinkMessage::Vaccine(vec![VaccineItem {
            ack_secret: [0xBA; 16],
            expiry_min: NOW + 60,
        }]),
        &mut relay.store,
        NOW,
    );
    assert!(relay.store.get(&[8; 16]).unwrap().is_some());
    assert!(relay.store.vaccines(NOW, 10).unwrap().is_empty());
}

#[test]
fn maintenance_expires_and_respray_once() {
    let mut alice = Node::new(&[]);
    own(&mut alice.store, envelope(9, C_TAG, 1, SECRET));
    let quarter = NOW + 72 * 60 / 4;
    assert!(
        maintain(&mut alice.store, &alice.policy, quarter - 1)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        alice
            .store
            .get(&[9; 16])
            .unwrap()
            .unwrap()
            .envelope
            .header
            .copy_budget,
        1
    );
    maintain(&mut alice.store, &alice.policy, quarter).unwrap();
    let s = alice.store.get(&[9; 16]).unwrap().unwrap();
    assert!(s.retried && s.envelope.header.copy_budget == alice.policy.initial_budget());
    alice.store.set_budget(&[9; 16], 1).unwrap();
    maintain(&mut alice.store, &alice.policy, quarter + 10).unwrap();
    assert_eq!(
        alice
            .store
            .get(&[9; 16])
            .unwrap()
            .unwrap()
            .envelope
            .header
            .copy_budget,
        1,
        "re-spray happens once"
    );
    assert_eq!(
        maintain(&mut alice.store, &alice.policy, NOW + 72 * 60).unwrap(),
        vec![Event::OwnExpired([9; 16])]
    );
}

#[test]
fn incompatible_peer_is_closed() {
    let mut node = Node::new(&[]);
    let mut s = LinkSession::new(node.policy);
    let hello = Hello {
        link_versions: vec![9],
        max_frame: 100,
        relay_class: RelayClass::AlwaysOn,
        relay_willing: true,
        free_capacity_bucket: 0,
        l2cap_psm: None,
        features: 0,
    };
    let (out, ev) = s.handle(LinkMessage::Hello(hello), &mut node.store, NOW);
    assert_eq!(out, vec![LinkMessage::Bye(1)]);
    assert_eq!(ev, vec![Event::PeerIncompatible]);
    assert!(s.is_closed());
}
