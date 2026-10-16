//! Features added for the app checklist (2026-10-16): ID codes, contact
//! rename/delete, re-pairing, resend, carry cap and attachments (ADR-026).
//! Real link sessions, SQLCipher and Olm; only the radio is simulated.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

mod common;
use common::*;
use vl_core::*;

const MTU: usize = 185;

#[test]
fn id_codes_match_across_phones_and_differ_between_people() {
    let (mut alice, mut bob) = (device("id-a"), device("id-b"));
    let (bob_c, _) = pair(&mut alice, "Alice", &mut bob, "Bob");
    let bob_seen_by_alice = alice
        .node
        .contacts()
        .unwrap()
        .into_iter()
        .find(|c| c.contact_id == bob_c)
        .unwrap();
    let code = bob.node.my_id_code().unwrap();
    assert_eq!(code.len(), 32);
    assert_eq!(bob_seen_by_alice.id_code, code);
    assert_ne!(alice.node.my_id_code().unwrap(), code);
}

#[test]
fn rename_and_delete_contact() {
    let (mut alice, mut bob) = (device("rd-a"), device("rd-b"));
    let (bob_c, _) = pair(&mut alice, "Alice", &mut bob, "Bob");
    alice.node.rename_contact(&bob_c, "  Bobby  ").unwrap();
    assert_eq!(alice.node.contacts().unwrap()[0].display_name, "Bobby");
    assert!(alice.node.rename_contact(&bob_c, "   ").is_err());

    alice.node.send_text(&bob_c, "findable words", T0).unwrap();
    assert_eq!(alice.node.search("findable", None, 10).unwrap().len(), 1);
    alice.node.delete_contact(&bob_c).unwrap();
    assert!(alice.node.contacts().unwrap().is_empty());
    assert!(alice.node.search("findable", None, 10).unwrap().is_empty());
    assert!(matches!(
        alice.node.send_text(&bob_c, "x", T0),
        Err(CoreError::UnknownContact)
    ));
    assert!(matches!(
        alice.node.delete_contact(&bob_c),
        Err(CoreError::UnknownContact)
    ));

    // Bob's next message is no longer recognised as mail for Alice.
    let a_in_b = bob.node.contacts().unwrap()[0].contact_id;
    bob.node
        .send_text(&a_in_b, "anyone there?", T0 + 5)
        .unwrap();
    encounter(&mut alice, &mut bob, T0 + 10, MTU);
    assert!(alice.node.conversation_summaries().unwrap().is_empty());
}

#[test]
fn re_pairing_in_person_refreshes_keys_and_keeps_history() {
    let (mut alice, mut bob) = (device("rp-a"), device("rp-b"));
    let (bob_c, alice_c) = pair(&mut alice, "Alice", &mut bob, "Bob");
    alice.node.set_verified(&bob_c, true).unwrap();
    alice.node.send_text(&bob_c, "before", T0).unwrap();
    encounter(&mut alice, &mut bob, T0 + 10, MTU);

    // Meet again and scan each other: same identities, fresh keys.
    let (again_b, again_a) = pair(&mut alice, "Alice", &mut bob, "Bob");
    assert_eq!(
        (again_b, again_a),
        (bob_c, alice_c),
        "same contacts, not duplicates"
    );
    let bob_now = &alice.node.contacts().unwrap()[0];
    assert!(bob_now.verified, "verification survives re-pairing");
    assert_eq!(alice.node.conversation(&bob_c, 10).unwrap().len(), 1);

    let after = alice.node.send_text(&bob_c, "after", T0 + 100).unwrap();
    let reply = bob.node.send_text(&alice_c, "got it", T0 + 101).unwrap();
    encounter(&mut alice, &mut bob, T0 + 110, MTU);
    encounter(&mut alice, &mut bob, T0 + 120, MTU);
    assert_eq!(bob.node.message(&after).unwrap().unwrap().body, "after");
    assert_eq!(alice.node.message(&reply).unwrap().unwrap().body, "got it");
    assert_eq!(state(&alice, &bob_c, &after), MessageState::Acknowledged);
}

#[test]
fn resend_after_expiry_is_delivered_once() {
    let (mut alice, mut bob) = (device("rs-a"), device("rs-b"));
    let (bob_c, alice_c) = pair(&mut alice, "Alice", &mut bob, "Bob");
    let m = alice.node.send_text(&bob_c, "are you there", T0).unwrap();
    assert!(matches!(
        alice.node.resend(&m, T0),
        Err(CoreError::CannotResend)
    ));
    let later = T0 + 8 * 24 * 3600;
    alice.node.maintain(later).unwrap();
    assert_eq!(state(&alice, &bob_c, &m), MessageState::Expired);

    alice.node.resend(&m, later).unwrap();
    assert_eq!(state(&alice, &bob_c, &m), MessageState::Queued);
    // Phones run maintenance on every wake; it rotates the 6-hour address tags.
    bob.node.maintain(later).unwrap();
    encounter(&mut alice, &mut bob, later + 10, MTU);
    assert_eq!(state(&alice, &bob_c, &m), MessageState::Acknowledged);
    let received: Vec<_> = bob
        .node
        .conversation(&alice_c, 10)
        .unwrap()
        .into_iter()
        .filter(|r| r.message_id == m)
        .collect();
    assert_eq!(received.len(), 1, "same message id, one copy");
    assert_eq!(received[0].body, "are you there");
}

#[test]
fn carry_cap_is_clamped() {
    let mut d = device("cap");
    d.node.set_carry_cap(1);
    assert_eq!(d.node.carry_cap(), 1 << 20);
    d.node.set_carry_cap(u64::MAX);
    assert_eq!(d.node.carry_cap(), 100 << 20);
    d.node.set_carry_cap(20 << 20);
    assert_eq!(d.node.carry_cap(), 20 << 20);
}

fn bytes(n: usize, seed: u8) -> Vec<u8> {
    (0..n)
        .map(|i| (i as u8).wrapping_mul(31).wrapping_add(seed))
        .collect()
}

#[test]
fn image_attachment_arrives_complete_and_its_delivery_is_proven() {
    let (mut alice, mut bob) = (device("im-a"), device("im-b"));
    let (bob_c, alice_c) = pair(&mut alice, "Alice", &mut bob, "Bob");
    let photo = bytes(5000, 7);
    let m = alice
        .node
        .send_attachment(
            &bob_c,
            ATTACHMENT_IMAGE,
            "gate.jpg",
            &photo,
            "the north gate",
            None,
            T0,
        )
        .unwrap();
    assert_eq!(alice.node.attachment_delivery(&m).unwrap(), (0, 4));

    encounter(&mut alice, &mut bob, T0 + 10, MTU);
    let got = bob.node.attachment(&m).unwrap().unwrap();
    assert!(got.complete && !got.outgoing);
    assert_eq!(
        (got.kind, got.name.as_str(), got.total_len),
        (ATTACHMENT_IMAGE, "gate.jpg", 5000)
    );
    assert_eq!(bob.node.attachment_data(&m).unwrap().unwrap(), photo);
    assert_eq!(
        bob.node.message(&m).unwrap().unwrap().body,
        "the north gate"
    );
    assert_eq!(
        bob.node.conversation_summaries().unwrap()[0]
            .contact
            .contact_id,
        alice_c
    );

    assert_eq!(state(&alice, &bob_c, &m), MessageState::Acknowledged);
    assert_eq!(alice.node.attachment_delivery(&m).unwrap(), (4, 4));
}

#[test]
fn largest_file_is_delivered_directly_in_one_meeting() {
    let (mut alice, mut bob) = (device("fl-a"), device("fl-b"));
    let (bob_c, _) = pair(&mut alice, "Alice", &mut bob, "Bob");
    let file = bytes(MAX_FILE_BYTES as usize, 3);
    let m = alice
        .node
        .send_attachment(&bob_c, ATTACHMENT_FILE, "notes.pdf", &file, "", None, T0)
        .unwrap();
    encounter(&mut alice, &mut bob, T0 + 60, MTU);
    let got = bob.node.attachment(&m).unwrap();
    assert_eq!(bob.node.attachment_data(&m).unwrap().unwrap(), file);
}

#[test]
fn file_through_a_relay_resumes_over_several_rounds() {
    let (mut alice, mut bob, mut relay) = (device("rl-a"), device("rl-b"), device("rl-r"));
    let (bob_c, _) = pair(&mut alice, "Alice", &mut bob, "Bob");
    let file = bytes(20_000, 9);
    let m = alice
        .node
        .send_attachment(&bob_c, ATTACHMENT_FILE, "map.png", &file, "", None, T0)
        .unwrap();
    let mut rounds = 0u64;
    while !bob.node.attachment(&m).unwrap().is_some_and(|a| a.complete) {
        rounds += 1;
        assert!(rounds <= 10, "should finish within a few relay rounds");
        encounter(&mut alice, &mut relay, T0 + 600 * rounds, MTU);
        encounter(&mut relay, &mut bob, T0 + 600 * rounds + 300, MTU);
    }
    assert!(
        rounds > 1,
        "the relay's per-recipient cap (8) spreads 15 envelopes over rounds"
    );
    assert_eq!(bob.node.attachment_data(&m).unwrap().unwrap(), file);
}

#[test]
fn attachments_reject_bad_sizes_and_cannot_be_resent() {
    let (mut alice, mut bob) = (device("bad-a"), device("bad-b"));
    let (bob_c, _) = pair(&mut alice, "Alice", &mut bob, "Bob");
    for (kind, len) in [
        (ATTACHMENT_IMAGE, MAX_IMAGE_BYTES as usize + 1),
        (ATTACHMENT_FILE, MAX_FILE_BYTES as usize + 1),
        (ATTACHMENT_FILE, 0),
        (9, 10),
    ] {
        assert!(matches!(
            alice
                .node
                .send_attachment(&bob_c, kind, "x", &bytes(len, 1), "", None, T0),
            Err(CoreError::InvalidAttachment)
        ));
    }
    let m = alice
        .node
        .send_attachment(&bob_c, ATTACHMENT_FILE, "a.txt", b"hello", "", None, T0)
        .unwrap();
    alice.node.maintain(T0 + 8 * 24 * 3600).unwrap();
    assert!(matches!(
        alice.node.resend(&m, T0 + 8 * 24 * 3600),
        Err(CoreError::CannotResend)
    ));
}

#[test]
fn deleting_an_attachment_message_drops_its_bytes() {
    let (mut alice, mut bob) = (device("del-a"), device("del-b"));
    let (bob_c, _) = pair(&mut alice, "Alice", &mut bob, "Bob");
    let m = alice
        .node
        .send_attachment(
            &bob_c,
            ATTACHMENT_FILE,
            "a.txt",
            b"secret plan",
            "",
            None,
            T0,
        )
        .unwrap();
    encounter(&mut alice, &mut bob, T0 + 10, MTU);
    assert!(bob.node.attachment_data(&m).unwrap().is_some());
    alice.node.delete_for_everyone(&bob_c, &m, T0 + 20).unwrap();
    encounter(&mut alice, &mut bob, T0 + 30, MTU);
    assert!(bob.node.attachment(&m).unwrap().is_none());
    assert!(bob.node.attachment_data(&m).unwrap().is_none());
}
