//! End-to-end conversation features over real link sessions
//! (research/messaging-apps/08). Only the radio is simulated.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

mod common;
use common::*;
use vl_core::*;

const MTU: usize = 185;

fn msg(d: &Device, id: &[u8; 16]) -> MessageRow {
    d.node.message(id).unwrap().unwrap()
}

// AC-03.1/.2, AC-04.2, AC-10.3, AC-05.2, AC-06.1 through the whole stack.
#[test]
fn reply_react_edit_delete_round_trip() {
    let (mut alice, mut bob) = (device("f-alice"), device("f-bob"));
    let (bob_c, alice_c) = pair(&mut alice, "Alice", &mut bob, "Bob");

    let first = alice.node.send_text(&bob_c, "lunch at noon?", T0).unwrap();
    encounter(&mut alice, &mut bob, T0 + 10, MTU);
    assert_eq!(msg(&bob, &first).body, "lunch at noon?");

    let reply = bob
        .node
        .send_message(
            &alice_c,
            "yes, north gate",
            SendOptions {
                reply_to: Some(first),
                forwarded: false,
            },
            T0 + 20,
        )
        .unwrap();
    bob.node.react(&alice_c, &first, 6, T0 + 21).unwrap();
    alice
        .node
        .edit_message(&bob_c, &first, "lunch at 12:30?", T0 + 22)
        .unwrap();
    let fwd = alice
        .node
        .send_message(
            &bob_c,
            "fwd: meeting moved",
            SendOptions {
                reply_to: None,
                forwarded: true,
            },
            T0 + 22,
        )
        .unwrap();
    let doomed = alice
        .node
        .send_text(&bob_c, "wrong chat, ignore", T0 + 23)
        .unwrap();
    alice
        .node
        .delete_for_everyone(&bob_c, &doomed, T0 + 24)
        .unwrap();

    encounter(&mut alice, &mut bob, T0 + 30, MTU);

    let r = msg(&alice, &reply);
    assert_eq!(r.reply_to, Some(first), "only the target id travelled");
    assert_eq!(r.body, "yes, north gate");
    assert_eq!(
        alice.node.reactions(&first).unwrap(),
        vec![Reaction {
            from_me: false,
            reaction: 6
        }]
    );
    let edited = msg(&bob, &first);
    assert_eq!(
        (edited.body.as_str(), edited.edited),
        ("lunch at 12:30?", true)
    );
    let f = msg(&bob, &fwd);
    assert!(
        f.forwarded && f.reply_to.is_none(),
        "forwarded flag only, no origin"
    );
    let gone = msg(&bob, &doomed);
    assert!(gone.deleted && gone.body.is_empty());
    assert!(msg(&alice, &doomed).deleted, "sender shows placeholder too");
    assert_eq!(bob.node.search("12:30", None, 10).unwrap().len(), 1);
    assert!(bob.node.search("ignore", None, 10).unwrap().is_empty());

    bob.node.react(&alice_c, &first, 0, T0 + 40).unwrap();
    encounter(&mut alice, &mut bob, T0 + 50, MTU);
    assert!(
        alice.node.reactions(&first).unwrap().is_empty(),
        "reaction removed"
    );

    assert_eq!(
        bob.node
            .edit_message(&alice_c, &first, "not mine", T0 + 60)
            .err()
            .map(|e| e.to_string()),
        Some(CoreError::NotAllowed.to_string()),
        "cannot edit the contact's message"
    );
    assert!(matches!(
        alice.node.react(&bob_c, &first, 9, T0 + 61),
        Err(CoreError::InvalidReaction)
    ));
}

// AC-05.3: the delete reaches the recipient before the message, through the
// real spray-and-wait engine, and the late original is stored as deleted.
#[test]
fn delete_arriving_before_its_target_via_relays() {
    let mut alice = device("o-alice");
    let mut bob = device("o-bob");
    let mut r = [
        device("o-r1"),
        device("o-r2"),
        device("o-r3"),
        device("o-r4"),
    ];
    let (bob_c, _) = pair(&mut alice, "Alice", &mut bob, "Bob");

    let m = alice.node.send_text(&bob_c, "sensitive plan", T0).unwrap();
    for relay in r.iter_mut().take(3) {
        encounter(&mut alice, relay, T0 + 10, MTU);
    }
    alice.node.delete_for_everyone(&bob_c, &m, T0 + 20).unwrap();
    encounter(&mut alice, &mut r[3], T0 + 30, MTU);

    encounter(&mut r[3], &mut bob, T0 + 40, MTU);
    assert!(
        bob.node.message(&m).unwrap().is_none(),
        "relay 4 carried only the delete"
    );
    encounter(&mut r[0], &mut bob, T0 + 50, MTU);
    let late = msg(&bob, &m);
    assert!(
        late.deleted && late.body.is_empty(),
        "late original stored as deleted"
    );
    assert!(bob.node.search("sensitive", None, 10).unwrap().is_empty());
}

// AC-08.1 .. AC-08.5
#[test]
fn blocking_drops_their_mail_but_keeps_relaying_for_others() {
    let (mut alice, mut bob, mut charlie) =
        (device("b-alice"), device("b-bob"), device("b-charlie"));
    let (bob_c, alice_in_bob) = pair(&mut alice, "Alice", &mut bob, "Bob");
    let (charlie_c, _) = pair(&mut alice, "Alice", &mut charlie, "Charlie");

    bob.node.set_blocked(&alice_in_bob, true, T0).unwrap();
    assert!(bob.node.contacts().unwrap()[0].blocked);
    assert!(matches!(
        bob.node.send_text(&alice_in_bob, "hi", T0),
        Err(CoreError::ContactBlocked)
    ));

    let to_bob = alice.node.send_text(&bob_c, "are you there?", T0).unwrap();
    let to_charlie = alice.node.send_text(&charlie_c, "for charlie", T0).unwrap();
    encounter(&mut alice, &mut bob, T0 + 10, MTU);
    assert!(bob.node.message(&to_bob).unwrap().is_none(), "dropped");
    assert_ne!(
        state(&alice, &bob_c, &to_bob),
        MessageState::Acknowledged,
        "no receipt"
    );

    encounter(&mut bob, &mut charlie, T0 + 20, MTU);
    assert!(
        charlie.node.message(&to_charlie).unwrap().is_some(),
        "bob still relayed for charlie"
    );

    bob.node.set_blocked(&alice_in_bob, false, T0 + 30).unwrap();
    let after = alice
        .node
        .send_text(&bob_c, "after unblock", T0 + 35)
        .unwrap();
    encounter(&mut alice, &mut bob, T0 + 40, MTU);
    assert!(
        bob.node.message(&after).unwrap().is_some(),
        "new mail flows after unblock"
    );
    assert!(
        bob.node.message(&to_bob).unwrap().is_none(),
        "mail sent while blocked stays undelivered (mainstream semantics)"
    );
}

// AC-09.2 .. AC-09.4
#[test]
fn disappearing_messages_follow_the_newest_timer_and_vanish() {
    let (mut alice, mut bob) = (device("d-alice"), device("d-bob"));
    let (bob_c, alice_c) = pair(&mut alice, "Alice", &mut bob, "Bob");

    alice.node.set_disappearing_timer(&bob_c, 3600, T0).unwrap();
    let m = alice
        .node
        .send_text(&bob_c, "self-destructing", T0 + 1)
        .unwrap();
    encounter(&mut alice, &mut bob, T0 + 10, MTU);
    assert_eq!(
        bob.node.disappearing_timer(&alice_c).unwrap(),
        3600,
        "adopted"
    );
    assert_eq!(msg(&bob, &m).timer_secs, 3600);
    assert!(matches!(
        bob.node.set_disappearing_timer(&alice_c, 30, T0),
        Err(CoreError::InvalidTimer)
    ));

    alice.node.maintain(T0 + 1 + 3600).unwrap();
    assert!(
        alice.node.message(&m).unwrap().is_none(),
        "sender copy gone at send + timer"
    );
    bob.node.maintain(T0 + 1 + 3600).unwrap();
    assert!(
        bob.node.message(&m).unwrap().is_some(),
        "unread copy waits for reading"
    );
    bob.node.mark_read(&alice_c, T0 + 5000).unwrap();
    bob.node.maintain(T0 + 5000 + 3600).unwrap();
    assert!(
        bob.node.message(&m).unwrap().is_none(),
        "gone at read + timer"
    );

    bob.node
        .set_disappearing_timer(&alice_c, 0, T0 + 9000)
        .unwrap();
    encounter(&mut alice, &mut bob, T0 + 9010, MTU);
    assert_eq!(
        alice.node.disappearing_timer(&bob_c).unwrap(),
        0,
        "newest change wins"
    );
}

// AC-W.2: a newer sender's unknown content type is acknowledged, not stored.
#[test]
fn unknown_content_type_is_acknowledged_and_ignored() {
    let (mut alice, mut bob) = (device("u-alice"), device("u-bob"));
    let (bob_c, alice_c) = pair(&mut alice, "Alice", &mut bob, "Bob");
    let future = alice
        .node
        .send_raw_content(&bob_c, &[1, 0x7F, 1, 2, 3], T0)
        .unwrap();
    let normal = alice.node.send_text(&bob_c, "still works", T0 + 1).unwrap();
    encounter(&mut alice, &mut bob, T0 + 10, MTU);
    assert!(bob.node.message(&future).unwrap().is_none());
    assert_eq!(msg(&bob, &normal).body, "still works");
    assert!(bob.node.conversation(&alice_c, 10).unwrap().len() == 1);
    // The raw event has no message row on the sender; the normal one is acknowledged.
    assert_eq!(state(&alice, &bob_c, &normal), MessageState::Acknowledged);
}

// AC-07.1 and drafts cleared on send; pins (F-11)
#[test]
fn drafts_and_pins() {
    let (mut alice, mut bob) = (device("p-alice"), device("p-bob"));
    let (bob_c, _) = pair(&mut alice, "Alice", &mut bob, "Bob");
    alice.node.save_draft(&bob_c, "half a thought", T0).unwrap();
    assert_eq!(
        alice.node.draft(&bob_c).unwrap().as_deref(),
        Some("half a thought")
    );
    assert!(matches!(
        alice
            .node
            .save_draft(&bob_c, &"x".repeat(MAX_TEXT_BYTES + 1), T0),
        Err(CoreError::TextTooLong)
    ));
    let m = alice
        .node
        .send_text(&bob_c, "the whole thought", T0 + 1)
        .unwrap();
    assert_eq!(
        alice.node.draft(&bob_c).unwrap(),
        None,
        "sending clears the draft"
    );
    assert!(alice.node.set_pinned(&m, true).unwrap());
    assert_eq!(
        alice.node.flagged(Some(&bob_c), true).unwrap()[0].message_id,
        m
    );
}

// AC-16.1, AC-16.2, AC-16.6
#[test]
fn panic_wipe_removes_databases_and_identity() {
    let (mut alice, mut bob) = (device("w-alice"), device("w-bob"));
    let (bob_c, _) = pair(&mut alice, "Alice", &mut bob, "Bob");
    alice.node.send_text(&bob_c, "CANARY wipe me", T0).unwrap();
    encounter(&mut alice, &mut bob, T0 + 10, MTU);
    let files = |dir: &std::path::Path| -> Vec<String> {
        std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect()
    };
    assert!(
        files(&alice.dir).iter().any(|f| f.ends_with("-wal")),
        "WAL present before wipe"
    );

    let (_link, _) = alice.node.link_open(true).unwrap();
    let Device { node, dir } = alice;
    node.panic_wipe().unwrap();
    assert!(files(&dir).is_empty(), "left behind: {:?}", files(&dir));

    let (relay_db, conversation_db) = (dir.join("relay.db"), dir.join("conv.db"));
    let reopened = Node::open(&NodeConfig {
        relay_db: &relay_db,
        relay_key: [1; 32],
        conversation_db: &conversation_db,
        conversation_key: [2; 32],
        policy: vl_routing::Policy::default_spray(),
    })
    .unwrap();
    assert!(!reopened.has_identity(), "fresh node after wipe");
    assert!(reopened.contacts().is_err() || reopened.contacts().unwrap().is_empty());
    assert!(
        reopened.relay_store().all().unwrap().is_empty(),
        "carried mail gone"
    );
}

// Delayed acknowledgement: the sender's copy expires before the receipt comes
// back; authenticated proof must still turn the message Delivered.
#[test]
fn receipt_arriving_after_sender_expiry_still_proves_delivery() {
    let (mut alice, mut bob, mut charlie) =
        (device("x-alice"), device("x-bob"), device("x-charlie"));
    let (charlie_c, _) = pair(&mut alice, "Alice", &mut charlie, "Charlie");
    let m = alice.node.send_text(&charlie_c, "late proof", T0).unwrap();
    encounter(&mut alice, &mut bob, T0 + 10, MTU);
    encounter(&mut bob, &mut charlie, T0 + 48 * 3600, MTU);
    assert!(charlie.node.message(&m).unwrap().is_some());

    let after_expiry = T0 + 75 * 3600;
    alice.node.maintain(after_expiry).unwrap();
    assert_eq!(state(&alice, &charlie_c, &m), MessageState::DeliveryUnknown);
    encounter(&mut bob, &mut alice, after_expiry + 60, MTU);
    assert_eq!(
        state(&alice, &charlie_c, &m),
        MessageState::Acknowledged,
        "receipt with a fresh lifetime proves delivery after expiry"
    );
}

// Causality under clock skew: Alice's clock runs 30 min fast. Bob changes the
// shared timer after seeing hers; his later change must win on both devices.
#[test]
fn timer_change_made_after_seeing_a_skewed_change_wins() {
    let (mut alice, mut bob) = (device("h-alice"), device("h-bob"));
    let (bob_c, alice_c) = pair(&mut alice, "Alice", &mut bob, "Bob");
    let skew = 30 * 60;
    alice
        .node
        .set_disappearing_timer(&bob_c, 3600, T0 + skew)
        .unwrap();
    encounter(&mut alice, &mut bob, T0 + 10, MTU);
    assert_eq!(bob.node.disappearing_timer(&alice_c).unwrap(), 3600);

    bob.node
        .set_disappearing_timer(&alice_c, 0, T0 + 60)
        .unwrap();
    assert_eq!(
        bob.node.disappearing_timer(&alice_c).unwrap(),
        0,
        "my own later change applies locally"
    );
    encounter(&mut alice, &mut bob, T0 + 120, MTU);
    assert_eq!(
        alice.node.disappearing_timer(&bob_c).unwrap(),
        0,
        "and wins on the other device"
    );
}

// HLC bounds: a contact clock beyond MAX_CLOCK_DRIFT_MS is not absorbed, and a
// local clock that steps backwards still produces increasing timestamps.
#[test]
fn hybrid_clock_is_bounded_and_monotonic() {
    let (mut alice, mut bob) = (device("c-alice"), device("c-bob"));
    let (bob_c, alice_c) = pair(&mut alice, "Alice", &mut bob, "Bob");
    let far_future = T0 + 2 * 3600;
    let first = alice
        .node
        .send_text(&bob_c, "from the future", far_future)
        .unwrap();
    encounter(&mut alice, &mut bob, T0 + 10, MTU);
    let reply = bob.node.send_text(&alice_c, "now", T0 + 20).unwrap();
    let sent = msg(&bob, &reply).sent_at_ms;
    assert!(
        sent < i64::try_from(far_future).unwrap() * 1000,
        "2 h ahead exceeds the drift bound and is ignored"
    );

    bob.node.react(&alice_c, &first, 1, T0 + 100).unwrap();
    bob.node.react(&alice_c, &first, 2, T0 + 50).unwrap();
    assert_eq!(
        bob.node.reactions(&first).unwrap()[0].reaction,
        2,
        "clock stepped back, newest reaction still wins"
    );
    encounter(&mut alice, &mut bob, T0 + 200, MTU);
    assert_eq!(alice.node.reactions(&first).unwrap()[0].reaction, 2);
}
