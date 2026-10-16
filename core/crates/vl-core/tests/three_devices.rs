//! End-to-end: three complete nodes (real SQLCipher databases, real Noise links,
//! real Olm) exchanging bytes in BLE-sized chunks. Only the radio is simulated.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use vl_core::*;
use vl_store::MessageState;

mod common;
use common::*;

#[test]
fn alice_bob_charlie_time_separated_with_restarts() {
    let (mut alice, mut bob, mut charlie) = (device("alice"), device("bob"), device("charlie"));
    let (charlie_for_alice, alice_for_charlie) = pair(&mut alice, "Alice", &mut charlie, "Charlie");

    let canary = "CANARY north gate 06:00";
    let mid = alice
        .node
        .send_text(&charlie_for_alice, canary, T0)
        .unwrap();
    assert_eq!(
        state(&alice, &charlie_for_alice, &mid),
        MessageState::Queued
    );

    encounter(&mut alice, &mut bob, T0 + 60, 20);
    assert_eq!(
        state(&alice, &charlie_for_alice, &mid),
        MessageState::AcceptedByRelay,
        "relay custody is not delivery"
    );

    bob = reopen(bob);
    alice = reopen(alice);
    let bob_files = [bob.dir.join("relay.db"), bob.dir.join("relay.db-wal")];
    for f in bob_files {
        let bytes = std::fs::read(&f).unwrap_or_default();
        assert!(
            !bytes.windows(canary.len()).any(|w| w == canary.as_bytes()),
            "relay must never store plaintext"
        );
    }

    encounter(&mut bob, &mut charlie, T0 + 3600, 185);
    let inbox = charlie.node.conversation(&alice_for_charlie, 10).unwrap();
    assert_eq!(inbox.len(), 1);
    assert_eq!(inbox[0].body, canary);
    assert!(!inbox[0].outgoing);
    assert_eq!(
        state(&alice, &charlie_for_alice, &mid),
        MessageState::AcceptedByRelay
    );

    encounter(&mut charlie, &mut bob, T0 + 3700, 64);
    encounter(&mut bob, &mut alice, T0 + 7200, 100);
    assert_eq!(
        state(&alice, &charlie_for_alice, &mid),
        MessageState::Acknowledged,
        "authenticated proof travelled back"
    );

    encounter(&mut bob, &mut charlie, T0 + 7300, 185);
    assert_eq!(
        charlie
            .node
            .conversation(&alice_for_charlie, 10)
            .unwrap()
            .len(),
        1,
        "duplicates never shown twice"
    );
}

#[test]
fn direct_encounter_delivers_and_acknowledges_in_one_session() {
    let (mut alice, mut charlie) = (device("a2"), device("c2"));
    let (c, a) = pair(&mut alice, "Alice", &mut charlie, "Charlie");
    let mid = alice.node.send_text(&c, "hi", T0).unwrap();
    encounter(&mut alice, &mut charlie, T0 + 10, 512);
    assert_eq!(charlie.node.conversation(&a, 10).unwrap()[0].body, "hi");
    assert_eq!(state(&alice, &c, &mid), MessageState::Acknowledged);
}

#[test]
fn conversation_continues_after_reply_ratchets() {
    let (mut alice, mut charlie) = (device("a3"), device("c3"));
    let (c, a) = pair(&mut alice, "Alice", &mut charlie, "Charlie");
    alice.node.send_text(&c, "one", T0).unwrap();
    encounter(&mut alice, &mut charlie, T0 + 10, 185);
    charlie.node.send_text(&a, "two", T0 + 20).unwrap();
    alice.node.send_text(&c, "three", T0 + 30).unwrap();
    encounter(&mut charlie, &mut alice, T0 + 40, 185);
    let mut bodies: Vec<String> = alice
        .node
        .conversation(&c, 10)
        .unwrap()
        .into_iter()
        .map(|m| m.body)
        .collect();
    bodies.sort();
    assert_eq!(bodies, vec!["one", "three", "two"]);
    assert_eq!(charlie.node.conversation(&a, 10).unwrap().len(), 3);
}

#[test]
fn undelivered_message_expires_honestly() {
    let (mut alice, mut bob, mut charlie) = (device("a4"), device("b4"), device("c4"));
    let (c, _) = pair(&mut alice, "Alice", &mut charlie, "Charlie");
    let before = alice
        .node
        .send_text(&c, "queued before meeting bob", T0)
        .unwrap();
    encounter(&mut alice, &mut bob, T0 + 60, 185);
    let handed = alice
        .node
        .send_text(&c, "queued after meeting bob", T0 + 120)
        .unwrap();
    alice.node.maintain(T0 + 8 * 24 * 3600).unwrap();
    let rows = alice.node.conversation(&c, 10).unwrap();
    let first = rows.iter().find(|m| m.message_id == before).unwrap();
    let second = rows.iter().find(|m| m.message_id == handed).unwrap();
    assert_eq!(
        first.state,
        MessageState::DeliveryUnknown,
        "was accepted by bob, never confirmed"
    );
    assert_eq!(second.state, MessageState::Expired, "never left the device");
}

#[test]
fn malformed_bytes_close_the_link_without_panic() {
    let mut d = device("m5");
    let (id, _) = d.node.link_open(false).unwrap();
    assert!(d.node.link_receive(id, &[0xFF, 0xFF, 1, 2, 3], T0).is_err());
    assert!(!d.node.link_is_open(id));
    let (id2, _) = d.node.link_open(false).unwrap();
    assert!(
        d.node.link_receive(id2, &[0, 4, 1, 2, 3, 4], T0).is_err(),
        "garbage handshake rejected"
    );
    assert!(!d.node.link_is_open(id2));
}

#[test]
fn pairing_errors() {
    let (mut a, mut b) = (device("p6"), device("p7"));
    assert!(matches!(
        a.node.complete_pairing(&[1, 2, 3], T0),
        Err(CoreError::NoPairingInProgress)
    ));
    pair(&mut a, "A", &mut b, "B");
    let cb = b.node.pairing_card("B", T0).unwrap();
    a.node.pairing_card("A", T0).unwrap();
    // Scanning an existing contact again re-pairs (fresh keys) instead of
    // failing, and never creates a duplicate contact (changed 2026-10-16).
    let first = a.node.contacts().unwrap()[0].contact_id;
    let again = a.node.complete_pairing(&cb, T0).unwrap();
    assert_eq!(again.contact_id, first);
    assert_eq!(a.node.contacts().unwrap().len(), 1);
    let mut tampered = b.node.pairing_card("B", T0).unwrap();
    let last = tampered.len() - 1;
    tampered[last] ^= 1;
    let mut c = device("p8");
    c.node.pairing_card("C", T0).unwrap();
    assert!(c.node.complete_pairing(&tampered, T0).is_err());
    assert!(matches!(
        a.node.send_text(&[0; 16], "x", T0),
        Err(CoreError::UnknownContact)
    ));
    let contact = a.node.contacts().unwrap()[0].contact_id;
    assert!(matches!(
        a.node
            .send_text(&contact, &"x".repeat(MAX_TEXT_BYTES + 1), T0),
        Err(CoreError::TextTooLong)
    ));
}
