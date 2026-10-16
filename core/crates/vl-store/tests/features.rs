#![allow(clippy::unwrap_used, clippy::indexing_slicing)]
//! Storage-level rules for conversation features (research/messaging-apps/08).
//! Every merge rule is exercised in both arrival orders.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use vl_store::*;

const KEY: [u8; 32] = [0x24; 32];
const ALICE: [u8; 16] = [0xA1; 16];
const BOB: [u8; 16] = [0xB0; 16];
const HOUR_MS: i64 = 3_600_000;

fn fresh() -> ConversationDb {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "vl-features-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let mut db = ConversationDb::open(&dir.join("c.db"), &KEY).unwrap();
    for (id, r) in [(ALICE, 1u8), (BOB, 2u8)] {
        db.add_contact(&Contact {
            contact_id: id,
            contact_ref: [r; 16],
            display_name: format!("{r}"),
            ..Default::default()
        })
        .unwrap();
    }
    db
}

fn msg(id: u8, contact: [u8; 16], outgoing: bool, body: &str, sent_at_ms: i64) -> MessageRow {
    MessageRow {
        message_id: [id; 16],
        contact_id: contact,
        outgoing,
        body: body.into(),
        created_at: sent_at_ms / 1000,
        state: if outgoing {
            MessageState::Queued
        } else {
            MessageState::Received
        },
        sent_at_ms,
        ..Default::default()
    }
}

fn ev(target: u8, contact: [u8; 16], from_me: bool, sent_at_ms: i64) -> EventMeta {
    EventMeta {
        target: [target; 16],
        contact_id: contact,
        from_me,
        sent_at_ms,
        received_at: sent_at_ms / 1000,
    }
}

fn reaction_of(db: &ConversationDb, target: u8, from_me: bool) -> Option<u8> {
    db.reactions(&[target; 16])
        .unwrap()
        .into_iter()
        .find(|r| r.from_me == from_me)
        .map(|r| r.reaction)
}

// AC-04.2, AC-04.3, AC-04.4
#[test]
fn reactions_converge_regardless_of_arrival_order() {
    let t0 = 1_000_000;
    let events = [
        (t0 + 1, 3u8, [1u8; 16]),
        (t0 + 2, 6, [2; 16]),
        (t0 + 3, 0, [3; 16]),
        (t0 + 4, 2, [4; 16]),
    ];
    let orders: [&[usize]; 3] = [&[0, 1, 2, 3], &[3, 2, 1, 0], &[2, 0, 3, 1]];
    for order in orders {
        for target_first in [true, false] {
            let mut db = fresh();
            if target_first {
                db.insert_message(&msg(9, ALICE, true, "hi", t0)).unwrap();
            }
            for &i in order {
                let (at, r, id) = events[i];
                db.upsert_reaction(&ev(9, ALICE, false, at), r, &id)
                    .unwrap();
                db.upsert_reaction(&ev(9, ALICE, false, at), r, &id)
                    .unwrap();
            }
            if !target_first {
                assert!(db.reactions(&[9; 16]).unwrap().is_empty(), "no target yet");
                db.insert_message(&msg(9, ALICE, true, "hi", t0)).unwrap();
            }
            assert_eq!(reaction_of(&db, 9, false), Some(2), "order {order:?}");
        }
    }
}

// AC-04.2 removal is sticky against older events
#[test]
fn reaction_removal_is_not_resurrected_by_older_event() {
    let mut db = fresh();
    db.insert_message(&msg(9, ALICE, false, "hi", 100)).unwrap();
    db.upsert_reaction(&ev(9, ALICE, true, 300), 0, &[1; 16])
        .unwrap();
    db.upsert_reaction(&ev(9, ALICE, true, 200), 5, &[2; 16])
        .unwrap();
    assert_eq!(reaction_of(&db, 9, true), None);
}

// AC-04.5
#[test]
fn reaction_from_another_conversation_is_ignored() {
    let mut db = fresh();
    db.insert_message(&msg(9, ALICE, false, "hi", 100)).unwrap();
    assert!(
        !db.upsert_reaction(&ev(9, BOB, false, 200), 1, &[1; 16])
            .unwrap()
    );
    assert!(db.reactions(&[9; 16]).unwrap().is_empty());
}

// AC-05.2, AC-05.3, AC-05.4
#[test]
fn delete_for_everyone_rules() {
    let t0 = 10 * HOUR_MS;
    for tomb_first in [false, true] {
        let mut db = fresh();
        let apply = |db: &mut ConversationDb| {
            db.add_tombstone(&ev(1, ALICE, false, t0 + HOUR_MS))
                .unwrap();
        };
        if tomb_first {
            apply(&mut db);
        }
        db.insert_message(&msg(1, ALICE, false, "secret", t0))
            .unwrap();
        db.upsert_reaction(&ev(1, ALICE, true, t0 + 1), 1, &[1; 16])
            .unwrap();
        if !tomb_first {
            apply(&mut db);
        }
        let m = db.message(&[1; 16]).unwrap().unwrap();
        assert!(m.deleted, "tomb_first={tomb_first}");
        assert_eq!(m.body, "");
        assert!(db.reactions(&[1; 16]).unwrap().is_empty());
        assert!(db.search("secret", None, 10).unwrap().is_empty());
        apply(&mut db);
        assert!(db.message(&[1; 16]).unwrap().unwrap().deleted, "idempotent");
    }

    let mut db = fresh();
    db.insert_message(&msg(2, ALICE, true, "mine", t0)).unwrap();
    assert!(
        !db.add_tombstone(&ev(2, ALICE, false, t0 + 1)).unwrap(),
        "contact cannot delete my message"
    );
    db.insert_message(&msg(3, ALICE, false, "theirs", t0))
        .unwrap();
    assert!(
        !db.add_tombstone(&ev(3, BOB, false, t0 + 1)).unwrap(),
        "other contact cannot delete"
    );
    let late = t0 + DELETE_WINDOW_MS + 1;
    assert!(
        !db.add_tombstone(&ev(3, ALICE, false, late)).unwrap(),
        "outside window"
    );
    assert!(!db.message(&[2; 16]).unwrap().unwrap().deleted);
    assert!(!db.message(&[3; 16]).unwrap().unwrap().deleted);

    db.add_tombstone(&ev(4, ALICE, false, late)).unwrap();
    db.insert_message(&msg(4, ALICE, false, "old", t0)).unwrap();
    assert!(
        !db.message(&[4; 16]).unwrap().unwrap().deleted,
        "early tombstone outside window"
    );
}

// AC-05.1
#[test]
fn delete_for_me_is_local_and_suppresses_duplicates() {
    let mut db = fresh();
    let m = msg(5, ALICE, false, "local", 100);
    db.insert_message(&m).unwrap();
    db.delete_message(&[5; 16], 1).unwrap();
    assert!(db.message(&[5; 16]).unwrap().is_none());
    assert!(
        !db.insert_message(&m).unwrap(),
        "duplicate copy is not stored again"
    );
    assert!(db.search("local", None, 10).unwrap().is_empty());
}

// AC-10.1 .. AC-10.4
#[test]
fn edits_converge_and_respect_author_and_window() {
    let t0 = 5 * HOUR_MS;
    for edit_first in [false, true] {
        let mut db = fresh();
        let edits = [(2u8, "second"), (1, "first"), (3, "third")];
        if edit_first {
            for (seq, body) in edits {
                db.upsert_edit(&ev(7, ALICE, false, t0 + i64::from(seq)), seq, body)
                    .unwrap();
            }
        }
        db.insert_message(&msg(7, ALICE, false, "original", t0))
            .unwrap();
        if !edit_first {
            for (seq, body) in edits {
                db.upsert_edit(&ev(7, ALICE, false, t0 + i64::from(seq)), seq, body)
                    .unwrap();
            }
        }
        let m = db.message(&[7; 16]).unwrap().unwrap();
        assert_eq!(
            (m.body.as_str(), m.edited),
            ("third", true),
            "edit_first={edit_first}"
        );
        assert_eq!(db.search("third", None, 5).unwrap().len(), 1);
        assert!(db.search("original", None, 5).unwrap().is_empty());
    }

    let mut db = fresh();
    db.insert_message(&msg(8, ALICE, true, "mine", t0)).unwrap();
    db.upsert_edit(&ev(8, ALICE, false, t0 + 1), 1, "forged")
        .unwrap();
    assert_eq!(
        db.message(&[8; 16]).unwrap().unwrap().body,
        "mine",
        "contact cannot edit my message"
    );
    db.insert_message(&msg(9, ALICE, false, "theirs", t0))
        .unwrap();
    db.upsert_edit(&ev(9, ALICE, false, t0 + EDIT_WINDOW_MS + 1), 1, "late")
        .unwrap();
    assert_eq!(
        db.message(&[9; 16]).unwrap().unwrap().body,
        "theirs",
        "outside window"
    );
    db.add_tombstone(&ev(9, ALICE, false, t0 + 2)).unwrap();
    db.upsert_edit(&ev(9, ALICE, false, t0 + 3), 2, "after delete")
        .unwrap();
    assert_eq!(
        db.message(&[9; 16]).unwrap().unwrap().body,
        "",
        "edit of deleted message has no effect"
    );
}

// AC-06.1, AC-06.2
#[test]
fn search_is_prefix_case_insensitive_and_injection_safe() {
    let mut db = fresh();
    db.insert_message(&msg(1, ALICE, false, "Meet at the North gate", 1000))
        .unwrap();
    db.insert_message(&msg(2, BOB, true, "northern lights tonight", 2000))
        .unwrap();
    assert_eq!(db.search("nor", None, 10).unwrap().len(), 2);
    assert_eq!(db.search("NORTH gate", None, 10).unwrap().len(), 1);
    assert_eq!(db.search("nor", Some(&BOB), 10).unwrap().len(), 1);
    assert_eq!(db.search("nor", None, 1).unwrap().len(), 1);
    for hostile in [
        "\"",
        "*",
        "NEAR(north gate)",
        "-north",
        "body:north",
        "(",
        ")",
        "north\"OR\"x",
        "AND OR NOT",
        "^north",
        "{north}",
        "",
    ] {
        db.search(hostile, None, 10).unwrap();
    }
    assert_eq!(
        db.search("north)", None, 10).unwrap().len(),
        2,
        "punctuation is literal, not syntax"
    );
    assert!(
        db.search("NEAR(north", None, 10).unwrap().is_empty(),
        "NEAR is the literal word 'near', not the FTS5 operator"
    );
    assert_eq!(
        db.search("-north", None, 10).unwrap().len(),
        2,
        "'-' is not negation"
    );
}

// AC-07.1
#[test]
fn drafts_save_load_and_clear() {
    let mut db = fresh();
    assert_eq!(db.draft(&ALICE).unwrap(), None);
    db.save_draft(&ALICE, "half-written", 1).unwrap();
    db.save_draft(&ALICE, "half-written, more", 2).unwrap();
    assert_eq!(
        db.draft(&ALICE).unwrap().as_deref(),
        Some("half-written, more")
    );
    assert_eq!(db.draft(&BOB).unwrap(), None);
    db.save_draft(&ALICE, "", 3).unwrap();
    assert_eq!(db.draft(&ALICE).unwrap(), None);
}

// AC-09.2, AC-09.3, AC-09.4
#[test]
fn disappearing_timer_is_last_writer_wins_and_purges() {
    let mut db = fresh();
    assert!(db.set_timer(&ALICE, 3600, 2000).unwrap());
    assert!(
        !db.set_timer(&ALICE, 60, 1000).unwrap(),
        "older change ignored"
    );
    assert_eq!(db.timer(&ALICE).unwrap(), 3600);
    assert_eq!(db.timer(&BOB).unwrap(), 0);

    let mut out = msg(1, ALICE, true, "mine", 1_000_000);
    out.timer_secs = 3600;
    out.expire_at = Some(1000 + 3600);
    db.insert_message(&out).unwrap();
    let mut inc = msg(2, ALICE, false, "theirs", 1_000_000);
    inc.timer_secs = 60;
    db.insert_message(&inc).unwrap();
    db.upsert_reaction(&ev(2, ALICE, true, 1_000_001), 1, &[1; 16])
        .unwrap();

    assert_eq!(
        db.purge_expired(10_000).unwrap(),
        1,
        "unread incoming does not expire; outgoing does"
    );
    assert!(db.message(&[1; 16]).unwrap().is_none());
    assert!(db.message(&[2; 16]).unwrap().is_some());
    assert_eq!(db.mark_read(&ALICE, 20_000).unwrap(), 1);
    assert_eq!(
        db.message(&[2; 16]).unwrap().unwrap().expire_at,
        Some(20_060)
    );
    assert_eq!(db.purge_expired(20_059).unwrap(), 0);
    assert_eq!(db.purge_expired(20_060).unwrap(), 1);
    assert!(db.reactions(&[2; 16]).unwrap().is_empty());
    assert!(db.search("theirs", None, 5).unwrap().is_empty());
}

// R-NEW-4: orphan events expire and are capped per contact
#[test]
fn orphan_events_expire_and_are_capped() {
    let mut db = fresh();
    db.upsert_reaction(&ev(1, ALICE, false, 1000), 1, &[1; 16])
        .unwrap();
    db.purge_expired(1 + ORPHAN_TTL_SECS + 1).unwrap();
    db.insert_message(&msg(1, ALICE, true, "late", 500))
        .unwrap();
    assert!(
        db.reactions(&[1; 16]).unwrap().is_empty(),
        "expired orphan dropped"
    );

    let mut db = fresh();
    for i in 0..MAX_ORPHANS_PER_CONTACT {
        let mut target = [0u8; 16];
        target[..8].copy_from_slice(&i.to_be_bytes());
        let e = EventMeta {
            target,
            ..ev(0, ALICE, false, 10)
        };
        assert!(db.upsert_reaction(&e, 1, &[1; 16]).unwrap());
    }
    assert!(
        !db.upsert_reaction(&ev(0xEE, ALICE, false, 10), 1, &[1; 16])
            .unwrap(),
        "cap reached"
    );
    assert!(
        db.upsert_reaction(&ev(0xEE, BOB, false, 10), 1, &[1; 16])
            .unwrap(),
        "per contact"
    );
}

// F-11 pins and bookmarks (local only)
#[test]
fn pins_and_stars_are_local_flags() {
    let mut db = fresh();
    db.insert_message(&msg(1, ALICE, false, "a", 1000)).unwrap();
    db.insert_message(&msg(2, BOB, false, "b", 2000)).unwrap();
    assert!(db.set_pinned(&[1; 16], true).unwrap());
    assert!(db.set_starred(&[2; 16], true).unwrap());
    assert_eq!(db.flagged(Some(&ALICE), true).unwrap().len(), 1);
    assert_eq!(db.flagged(None, false).unwrap()[0].message_id, [2; 16]);
    db.add_tombstone(&ev(1, ALICE, false, 1001)).unwrap();
    assert!(
        db.flagged(None, true).unwrap().is_empty(),
        "deleting unpins"
    );
    assert!(!db.set_pinned(&[1; 16], true).unwrap());
}

#[test]
fn blocked_flag_round_trips() {
    let mut db = fresh();
    db.set_blocked(&ALICE, true).unwrap();
    let c = db.contacts().unwrap();
    assert!(c.iter().any(|c| c.contact_id == ALICE && c.blocked));
    assert!(c.iter().any(|c| c.contact_id == BOB && !c.blocked));
}

// Group messages that arrive before their roster are held, capped per contact,
// released by take, and expired with the orphan TTL.
#[test]
fn group_pending_is_bounded_and_expires() {
    let mut db = fresh();
    let pending = |i: u8, contact: [u8; 16]| PendingGroupMessage {
        contact_id: contact,
        epoch: 1,
        message: GroupMessageRow {
            group_msg_id: [i; 16],
            group_id: [0x61; 16],
            author_fp: [0; 32],
            outgoing: false,
            body: format!("m{i}"),
            reply_to: None,
            created_at: 10,
            sent_at_ms: 10_000,
        },
    };
    for i in 0..u8::try_from(MAX_GROUP_PENDING_PER_CONTACT).unwrap() {
        assert!(db.hold_group_message(&pending(i, ALICE), 10).unwrap());
    }
    assert!(
        !db.hold_group_message(&pending(200, ALICE), 10).unwrap(),
        "cap"
    );
    assert!(
        db.hold_group_message(&pending(201, BOB), 10).unwrap(),
        "per contact"
    );
    let taken = db.take_group_pending(&[0x61; 16]).unwrap();
    assert_eq!(
        taken.len(),
        usize::try_from(MAX_GROUP_PENDING_PER_CONTACT).unwrap() + 1
    );
    assert!(
        db.take_group_pending(&[0x61; 16]).unwrap().is_empty(),
        "taken once"
    );

    db.hold_group_message(&pending(1, ALICE), 10).unwrap();
    db.purge_expired(10 + ORPHAN_TTL_SECS + 1).unwrap();
    assert!(
        db.take_group_pending(&[0x61; 16]).unwrap().is_empty(),
        "expired"
    );
}

fn attachment_row(mid: u8, aid: u8, contact: [u8; 16], total_len: u32) -> AttachmentRow {
    AttachmentRow {
        message_id: [mid; 16],
        attachment_id: [aid; 16],
        contact_id: contact,
        outgoing: false,
        kind: 2,
        name: "f.bin".into(),
        total_len,
        sha256: [0; 32],
        key: [0; 32],
        received_chunks: 0,
        complete: false,
    }
}

// ADR-026: chunks may arrive before the manifest, in any order, twice.
#[test]
fn attachment_chunks_assemble_in_any_order_and_dedupe() {
    let mut db = fresh();
    let parts = [vec![1u8; 1400], vec![2u8; 1400], vec![3u8; 200]];
    db.add_chunk(&ALICE, &[7; 16], 2, &parts[2], 10).unwrap();
    db.add_chunk(&ALICE, &[7; 16], 0, &parts[0], 10).unwrap();
    assert!(
        !db.add_chunk(&ALICE, &[7; 16], 0, &parts[0], 11).unwrap(),
        "duplicate"
    );
    assert_eq!(db.assembled_attachment(&ALICE, &[7; 16], 3).unwrap(), None);
    db.insert_message(&msg(9, ALICE, false, "", 1000)).unwrap();
    db.put_attachment(&attachment_row(9, 7, ALICE, 3000), None)
        .unwrap();
    assert_eq!(db.attachment(&[9; 16]).unwrap().unwrap().received_chunks, 2);
    db.add_chunk(&ALICE, &[7; 16], 1, &parts[1], 12).unwrap();
    let pieces = db
        .assembled_attachment(&ALICE, &[7; 16], 3)
        .unwrap()
        .unwrap();
    assert_eq!(pieces, parts.to_vec(), "in index order");
    let all = pieces.concat();
    db.finish_attachment(&ALICE, &[7; 16], Some(&all)).unwrap();
    let row = db.attachment(&[9; 16]).unwrap().unwrap();
    assert!(row.complete);
    assert_eq!(row.received_chunks, 3);
    assert!(
        !db.add_chunk(&ALICE, &[7; 16], 0, &parts[0], 13).unwrap(),
        "already complete"
    );
    assert_eq!(db.attachment_data(&[9; 16]).unwrap().unwrap(), all);
}

// A chunk that fails verification is discarded, never stored as the file.
#[test]
fn failed_verification_keeps_no_bytes() {
    let mut db = fresh();
    db.insert_message(&msg(9, ALICE, false, "", 1000)).unwrap();
    db.put_attachment(&attachment_row(9, 7, ALICE, 10), None)
        .unwrap();
    db.add_chunk(&ALICE, &[7; 16], 0, &[0xEE; 10], 10).unwrap();
    db.finish_attachment(&ALICE, &[7; 16], None).unwrap();
    assert!(db.attachment_data(&[9; 16]).unwrap().is_none());
    assert_eq!(db.attachment(&[9; 16]).unwrap().unwrap().received_chunks, 0);
}

// Partial bytes per contact are bounded; other contacts are unaffected; expiry frees them.
#[test]
fn partial_attachment_quota_and_expiry() {
    let mut db = fresh();
    let chunk = vec![0u8; 1400];
    let mut stored = 0;
    for i in 0..400u16 {
        if db
            .add_chunk(&ALICE, &[(i / 40) as u8; 16], i % 40, &chunk, 0)
            .unwrap()
        {
            stored += 1;
        }
    }
    assert_eq!(stored, MAX_PARTIAL_ATTACHMENT_BYTES as usize / 1400);
    assert!(db.add_chunk(&BOB, &[1; 16], 0, &chunk, 0).unwrap());
    db.purge_expired(ORPHAN_TTL_SECS + 1).unwrap();
    assert!(
        db.add_chunk(&ALICE, &[99; 16], 0, &chunk, ORPHAN_TTL_SECS + 2)
            .unwrap()
    );
}

#[test]
fn rename_delete_contact_and_card_replacement() {
    let mut db = fresh();
    assert!(db.rename_contact(&ALICE, "Asha").unwrap());
    assert!(!db.rename_contact(&[0x33; 16], "x").unwrap());
    db.insert_message(&msg(1, ALICE, true, "hello there", 1000))
        .unwrap();
    db.save_session(&ALICE, "s1", "pickle", 1).unwrap();
    db.replace_contact_card(&ALICE, b"new card").unwrap();
    assert!(
        db.sessions(&ALICE).unwrap().is_empty(),
        "old sessions dropped"
    );
    assert_eq!(
        db.conversation(&ALICE, 10).unwrap().len(),
        1,
        "history kept"
    );
    assert_eq!(db.delete_contact(&ALICE).unwrap(), Some([1; 16]));
    assert!(db.message(&[1; 16]).unwrap().is_none());
    assert!(db.search("hello", None, 10).unwrap().is_empty());
    assert_eq!(db.delete_contact(&ALICE).unwrap(), None);
}
