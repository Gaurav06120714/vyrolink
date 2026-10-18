//! Contact presence over a link (ADR-029): paired phones in range see each
//! other; strangers, blocked contacts and peers without the feature learn nothing.
//! The last sighting per contact is kept (ADR-033).

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

mod common;
use common::*;
use vl_core::ARRIVAL_GAP_SECS;
use vl_crypto::{LinkHandshake, LinkTransport};
use vl_proto::{Deframer, Hello, LINK_VERSION, LinkMessage, MAX_FRAME_LEN, RelayClass, frame};

const MINUTE: u64 = 60;
const MTU: usize = 185;

fn ids(d: &Device, now: u64) -> Vec<[u8; 16]> {
    let mut v: Vec<_> = d
        .node
        .nearby_contacts(now, 10 * MINUTE)
        .unwrap()
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    v.sort_unstable();
    v
}

#[test]
fn paired_phones_in_range_see_each_other_and_it_fades() {
    let (mut alice, mut bob) = (device("nb-a"), device("nb-b"));
    let (bob_c, alice_c) = pair(&mut alice, "Alice", &mut bob, "Bob");
    assert!(ids(&alice, T0).is_empty());

    encounter(&mut alice, &mut bob, T0, MTU);
    assert_eq!(ids(&alice, T0), vec![bob_c]);
    assert_eq!(ids(&bob, T0), vec![alice_c]);
    let seen = alice.node.nearby_contacts(T0, 10 * MINUTE).unwrap()[0].1;
    assert_eq!(seen, T0);

    // One arrival each; meeting again soon after is not a new arrival.
    assert_eq!(alice.node.take_nearby_arrivals(), 1);
    assert_eq!(alice.node.take_nearby_arrivals(), 0);
    encounter(&mut alice, &mut bob, T0 + 5 * MINUTE, MTU);
    assert_eq!(alice.node.take_nearby_arrivals(), 0);
    // Back after the gap: a new arrival.
    let back = T0 + 5 * MINUTE + ARRIVAL_GAP_SECS;
    encounter(&mut alice, &mut bob, back, MTU);
    assert_eq!(alice.node.take_nearby_arrivals(), 1);
    assert_eq!(ids(&alice, back), vec![bob_c]);

    // Out of the window it is no longer "nearby".
    assert!(ids(&alice, back + 11 * MINUTE).is_empty());
    // "Nearby now" is memory only: a restart forgets it. The single
    // last-seen time per contact is kept, encrypted (ADR-033).
    let alice = reopen(alice);
    assert!(ids(&alice, T0).is_empty());
    let c = alice.node.contacts().unwrap();
    assert_eq!(c[0].last_seen_secs, Some(i64::try_from(back).unwrap()));
}

#[test]
fn strangers_and_blocked_contacts_are_not_reported() {
    let (mut alice, mut bob, mut carol) = (device("nb2-a"), device("nb2-b"), device("nb2-c"));
    let (bob_c, alice_c) = pair(&mut alice, "Alice", &mut bob, "Bob");

    // Carol is not paired with Alice: an encounter reveals nobody to either.
    encounter(&mut alice, &mut carol, T0, MTU);
    assert!(ids(&alice, T0).is_empty());
    assert!(ids(&carol, T0).is_empty());

    // Bob blocked Alice: he neither shows her his tag nor records her.
    bob.node.set_blocked(&alice_c, true, T0).unwrap();
    encounter(&mut alice, &mut bob, T0 + MINUTE, MTU);
    assert!(ids(&bob, T0 + MINUTE).is_empty());
    assert!(ids(&alice, T0 + MINUTE).is_empty());

    bob.node
        .set_blocked(&alice_c, false, T0 + 2 * MINUTE)
        .unwrap();
    encounter(&mut alice, &mut bob, T0 + 2 * MINUTE, MTU);
    assert_eq!(ids(&alice, T0 + 2 * MINUTE), vec![bob_c]);

    // Deleting the contact forgets the sighting at once.
    alice.node.delete_contact(&bob_c).unwrap();
    assert!(ids(&alice, T0 + 2 * MINUTE).is_empty());
}

/// Drives the far end of a link by hand, like a phone from before ADR-029.
struct OldPeer {
    t: LinkTransport,
    deframer: Deframer,
}

impl OldPeer {
    fn connect(d: &mut Device, link: u64) -> Self {
        let mut hs = LinkHandshake::initiator().unwrap();
        let reply = d
            .node
            .link_receive(link, &frame(&hs.write_message().unwrap()).unwrap(), T0)
            .unwrap();
        let mut deframer = Deframer::new(MAX_FRAME_LEN + 16);
        let frames = deframer.push(&reply).unwrap();
        hs.read_message(&frames[0]).unwrap();
        assert!(hs.is_finished());
        let mut t = hs.into_transport().unwrap();
        // The responder's HELLO follows the handshake in the same reply.
        for f in &frames[1..] {
            let m = LinkMessage::decode(&t.decrypt(f).unwrap()).unwrap();
            assert!(!matches!(m, LinkMessage::Presence(_)));
        }
        OldPeer { t, deframer }
    }

    fn send(&mut self, d: &mut Device, link: u64, m: &LinkMessage) -> Vec<LinkMessage> {
        let bytes = frame(&self.t.encrypt(&m.encode().unwrap()).unwrap()).unwrap();
        let reply = d.node.link_receive(link, &bytes, T0).unwrap();
        self.deframer
            .push(&reply)
            .unwrap()
            .iter()
            .map(|f| LinkMessage::decode(&self.t.decrypt(f).unwrap()).unwrap())
            .collect()
    }
}

#[test]
fn presence_goes_only_to_peers_that_advertise_it_and_is_padded() {
    let (mut alice, mut bob) = (device("nb3-a"), device("nb3-b"));
    pair(&mut alice, "Alice", &mut bob, "Bob");

    for features in [0, vl_proto::FEATURE_PRESENCE] {
        let (link, _) = alice.node.link_open(false).unwrap();
        let mut peer = OldPeer::connect(&mut alice, link);
        let hello = LinkMessage::Hello(Hello {
            link_versions: vec![LINK_VERSION],
            max_frame: MAX_FRAME_LEN as u16,
            relay_class: RelayClass::BestEffort,
            relay_willing: true,
            free_capacity_bucket: 255,
            l2cap_psm: None,
            features,
        });
        let replies = peer.send(&mut alice, link, &hello);
        let presence: Vec<_> = replies
            .iter()
            .filter_map(|m| match m {
                LinkMessage::Presence(tags) => Some(tags.len()),
                _ => None,
            })
            .collect();
        if features == 0 {
            assert!(presence.is_empty(), "an old peer must never get PRESENCE");
        } else {
            // One real contact, padded to a full block of 16.
            assert_eq!(presence, vec![vl_proto::PRESENCE_PAD]);
        }
        alice.node.link_close(link);
    }
    // A hand-made peer has no matching tags, so nothing is recorded.
    assert!(ids(&alice, T0).is_empty());
}
