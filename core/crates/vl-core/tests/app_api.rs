//! Node APIs the mobile app's screens use: QR pairing codes, chat-list
//! summaries, the pending queue, delivery timelines, settings and counters.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

mod common;
use common::*;
use vl_core::*;

const MTU: usize = 185;

#[test]
fn pairing_by_qr_text_and_self_scan_is_refused() {
    let (mut alice, mut bob) = (device("q-alice"), device("q-bob"));
    let code_a = alice.node.pairing_code("Alice", T0).unwrap();
    let code_b = bob.node.pairing_code("Bob", T0).unwrap();
    assert!(code_a.starts_with("VL1:"));
    assert!(
        code_a
            .bytes()
            .all(|c| b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:".contains(&c)),
        "code must stay in the QR alphanumeric set"
    );

    assert!(matches!(
        alice.node.complete_pairing_code(&code_a, T0),
        Err(CoreError::OwnPairingCode)
    ));
    assert!(matches!(
        alice.node.complete_pairing_code("https://example.com", T0),
        Err(CoreError::Proto(_))
    ));

    let bob_in_a = alice.node.complete_pairing_code(&code_b, T0).unwrap();
    let alice_in_b = bob
        .node
        .complete_pairing_code(&format!(" {code_a}\n"), T0)
        .unwrap();
    assert_eq!(bob_in_a.display_name, "Bob");
    assert_eq!(bob_in_a.safety_number, alice_in_b.safety_number);
    assert_eq!(bob_in_a.fingerprint, bob.node.my_fingerprint().unwrap());
    assert_eq!(alice_in_b.fingerprint, alice.node.my_fingerprint().unwrap());
    assert!(matches!(
        alice.node.complete_pairing_code(&code_b, T0),
        Err(CoreError::NoPairingInProgress | CoreError::AlreadyPaired)
    ));

    let m = alice
        .node
        .send_text(&bob_in_a.contact_id, "hi", T0 + 1)
        .unwrap();
    encounter(&mut alice, &mut bob, T0 + 5, MTU);
    assert_eq!(bob.node.message(&m).unwrap().unwrap().body, "hi");
}

#[test]
fn summaries_pending_timeline_and_stats_follow_real_delivery() {
    let (mut alice, mut bob) = (device("s-alice"), device("s-bob"));
    let (bob_c, alice_c) = pair(&mut alice, "Alice", &mut bob, "Bob");
    let mut carol = device("s-carol");
    let (carol_c, _) = pair(&mut alice, "Alice", &mut carol, "Carol");

    let summaries = alice.node.conversation_summaries().unwrap();
    assert_eq!(summaries.len(), 2);
    assert!(summaries.iter().all(|s| s.last.is_none() && s.unread == 0));

    let m1 = alice.node.send_text(&bob_c, "first", T0 + 1).unwrap();
    let m2 = alice.node.send_text(&carol_c, "newest", T0 + 2).unwrap();
    let summaries = alice.node.conversation_summaries().unwrap();
    assert_eq!(summaries[0].contact.contact_id, carol_c);
    assert_eq!(summaries[0].last.as_ref().unwrap().message_id, m2);
    let pending: Vec<_> = alice
        .node
        .pending_outgoing()
        .unwrap()
        .into_iter()
        .map(|m| m.message_id)
        .collect();
    assert_eq!(pending, vec![m2, m1]);
    assert_eq!(alice.node.stats().unwrap().own_envelopes, 2);

    encounter(&mut alice, &mut bob, T0 + 10, MTU);
    let bob_view = bob.node.conversation_summaries().unwrap();
    assert_eq!(bob_view[0].contact.contact_id, alice_c);
    assert_eq!(bob_view[0].unread, 1);
    bob.node.mark_read(&alice_c, T0 + 11).unwrap();
    assert_eq!(bob.node.conversation_summaries().unwrap()[0].unread, 0);

    assert_eq!(state(&alice, &bob_c, &m1), MessageState::Acknowledged);
    let pending: Vec<_> = alice
        .node
        .pending_outgoing()
        .unwrap()
        .into_iter()
        .map(|m| m.message_id)
        .collect();
    assert_eq!(pending, vec![m2], "a delivered message leaves the queue");
    let timeline: Vec<_> = alice
        .node
        .message_events(&m1)
        .unwrap()
        .into_iter()
        .map(|(_, s)| s)
        .collect();
    assert_eq!(timeline.first(), Some(&MessageState::Queued));
    assert_eq!(timeline.last(), Some(&MessageState::Acknowledged));
    assert_eq!(alice.node.stats().unwrap().open_links, 0);
}

#[test]
fn relay_counters_report_carried_mail() {
    let (mut alice, mut bob) = (device("r-alice"), device("r-bob"));
    let (bob_c, _) = pair(&mut alice, "Alice", &mut bob, "Bob");
    let mut relay = device("r-relay");
    alice.node.send_text(&bob_c, "via relay", T0 + 1).unwrap();
    encounter(&mut alice, &mut relay, T0 + 5, MTU);
    let stats = relay.node.stats().unwrap();
    assert_eq!(stats.carried_envelopes, 1);
    assert!(stats.carried_bytes > 0);
    assert_eq!(stats.own_envelopes, 0);
}

#[test]
fn settings_are_persisted_encrypted_and_survive_reopen() {
    let mut d = device("settings");
    assert_eq!(d.node.setting("profile.name").unwrap(), None);
    d.node.set_setting("profile.name", Some("Asha")).unwrap();
    d.node.set_setting("profile.name", Some("Asha K")).unwrap();
    let d = reopen(d);
    assert_eq!(
        d.node.setting("profile.name").unwrap().as_deref(),
        Some("Asha K")
    );
    let mut d = d;
    d.node.set_setting("profile.name", None).unwrap();
    assert_eq!(d.node.setting("profile.name").unwrap(), None);
}

// Reset when the keys are gone: the host deletes the files without a node.
#[test]
fn database_files_can_be_removed_without_opening_them() {
    let d = device("reset");
    let Device { node, dir } = d;
    drop(node);
    let paths = [dir.join("relay.db"), dir.join("conv.db")];
    assert!(paths.iter().all(|p| p.exists()));
    remove_database_files(&paths).unwrap();
    remove_database_files(&paths).unwrap();
    let left: Vec<_> = std::fs::read_dir(&dir).unwrap().collect();
    assert!(left.is_empty(), "left behind: {left:?}");
}
