#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use vl_crypto::ack_commitment;
use vl_proto::*;
use vl_routing::{MemStore, Store, Stored};
use vl_store::*;

const KEY: [u8; 32] = [0x42; 32];
const NOW: u32 = 29_000_000;

fn temp_db(name: &str) -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "vl-store-test-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn envelope(id: u8, tag: [u8; 16], secret: [u8; 16], expiry: u32) -> Envelope {
    Envelope {
        header: Header {
            kind: EnvelopeKind::Data,
            flags: FLAG_VACCINE,
            hop_count: 0,
            hop_limit: 8,
            copy_budget: 4,
            envelope_id: [id; 16],
            recipient_tag: tag,
            expiry_min: expiry,
            ack_commitment: ack_commitment(&secret),
            body_len: 256,
        },
        body: vec![id; 256],
    }
}

/// The same behavioural contract for every Store implementation.
fn conformance(store: &mut dyn Store) {
    let tag = [7; 16];
    store
        .insert(Stored {
            envelope: envelope(1, tag, [1; 16], NOW + 100),
            own: false,
            stored_at_min: NOW,
            retried: false,
        })
        .unwrap();
    store
        .insert(Stored {
            envelope: envelope(2, tag, [2; 16], NOW + 10),
            own: true,
            stored_at_min: NOW,
            retried: false,
        })
        .unwrap();
    assert_eq!(
        store.get(&[1; 16]).unwrap().unwrap().envelope,
        envelope(1, tag, [1; 16], NOW + 100)
    );
    assert_eq!(store.all().unwrap().len(), 2);
    assert_eq!(store.carried_bytes().unwrap(), HEADER_LEN + 256);
    assert_eq!(store.carried_for_tag(&tag).unwrap(), 1);

    store.set_budget(&[1; 16], 2).unwrap();
    assert_eq!(
        store
            .get(&[1; 16])
            .unwrap()
            .unwrap()
            .envelope
            .header
            .copy_budget,
        2
    );
    store.set_retried(&[2; 16]).unwrap();
    assert!(store.get(&[2; 16]).unwrap().unwrap().retried);

    assert!(!store.seen(&[9; 16]).unwrap());
    store.mark_seen(&[9; 16], NOW + 50).unwrap();
    assert!(store.seen(&[9; 16]).unwrap());

    assert_eq!(
        store.vaccine_holders(&ack_commitment(&[1; 16])).unwrap(),
        vec![([1; 16], false)]
    );
    assert_eq!(
        store.vaccine_holders(&ack_commitment(&[2; 16])).unwrap(),
        vec![([2; 16], true)]
    );
    assert!(store.vaccine_holders(&[0xEE; 16]).unwrap().is_empty());
    let item = VaccineItem {
        ack_secret: [1; 16],
        expiry_min: NOW + 100,
    };
    store.add_vaccine(item, ack_commitment(&[1; 16])).unwrap();
    assert!(
        store.get(&[1; 16]).unwrap().is_none(),
        "vaccine purges matching envelope"
    );
    assert!(store.has_vaccine(&ack_commitment(&[1; 16])).unwrap());
    assert_eq!(store.vaccines(NOW, 10).unwrap(), vec![item]);

    let expired = store.purge_expired(NOW + 10).unwrap();
    assert_eq!(expired.len(), 1);
    assert!(expired[0].own);
    assert!(store.get(&[2; 16]).unwrap().is_none());
    assert!(store.seen(&[9; 16]).unwrap(), "seen entry not yet expired");
    store.purge_expired(NOW + 100).unwrap();
    assert!(!store.seen(&[9; 16]).unwrap());
    assert!(store.vaccines(NOW, 10).unwrap().is_empty());
}

#[test]
fn mem_store_conformance() {
    conformance(&mut MemStore::default());
}

#[test]
fn relay_db_conformance() {
    let mut db = RelayDb::open(&temp_db("r.db"), &KEY).unwrap();
    conformance(&mut db);
}

#[test]
fn relay_db_persists_and_rejects_wrong_key() {
    let path = temp_db("persist.db");
    {
        let mut db = RelayDb::open(&path, &KEY).unwrap();
        db.insert(Stored {
            envelope: envelope(3, [3; 16], [3; 16], NOW + 99),
            own: true,
            stored_at_min: NOW,
            retried: false,
        })
        .unwrap();
        db.set_tags(&[0xC0; 16], &[(1, [0xAA; 16]), (2, [0xBB; 16])])
            .unwrap();
        db.put_pair_keys(&[0xC0; 16], &[0x5A; 128]).unwrap();
    }
    let db = RelayDb::open(&path, &KEY).unwrap();
    assert!(db.get(&[3; 16]).unwrap().unwrap().own);
    assert!(db.is_my_tag(&[0xBB; 16]).unwrap());
    assert_eq!(db.contact_for_tag(&[0xAA; 16]).unwrap(), Some([0xC0; 16]));
    assert_eq!(db.pair_keys(&[0xC0; 16]).unwrap(), Some([0x5A; 128]));
    drop(db);
    assert!(matches!(
        RelayDb::open(&path, &[0x43; 32]),
        Err(DbError::WrongKeyOrCorrupt)
    ));
}

fn file_contains(path: &PathBuf, needle: &[u8]) -> bool {
    let mut haystack = std::fs::read(path).unwrap_or_default();
    for suffix in ["-wal", "-shm"] {
        let mut p = path.clone().into_os_string();
        p.push(suffix);
        haystack.extend(std::fs::read(PathBuf::from(p)).unwrap_or_default());
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn plaintext_never_reaches_disk() {
    let path = temp_db("c.db");
    let canary = "CANARY-meet-at-the-north-gate-7f3a";
    let mut db = ConversationDb::open(&path, &KEY).unwrap();
    db.add_contact(&Contact {
        contact_id: [1; 16],
        contact_ref: [2; 16],
        display_name: "Charlie-CANARY".into(),
        card: vec![0; 10],
        verified: false,
        paired_at: 1,
        next_otk: 0,
        blocked: false,
    })
    .unwrap();
    db.insert_message(&MessageRow {
        message_id: [3; 16],
        contact_id: [1; 16],
        outgoing: true,
        body: canary.into(),
        created_at: 1,
        state: MessageState::Queued,
        envelope_id: None,
        ack_secret: None,
        expiry_min: None,
        ..Default::default()
    })
    .unwrap();
    db.save_draft(&[1; 16], "draft-CANARY-unsent", 2).unwrap();
    assert_eq!(db.search("CANARY", None, 5).unwrap().len(), 1);
    drop(db);
    assert!(!file_contains(&path, canary.as_bytes()));
    assert!(!file_contains(&path, b"Charlie-CANARY"));
    assert!(!file_contains(&path, b"draft-CANARY"));
    assert!(
        !file_contains(&path, b"north"),
        "search index must be encrypted"
    );
    let wal = path.with_extension("db-wal");
    assert!(!wal.exists() || !file_contains(&wal, b"north"));
    assert!(
        !file_contains(&path, b"SQLite format 3"),
        "SQLCipher must not leave a plain SQLite header"
    );
}

#[test]
fn delivered_state_cannot_be_downgraded() {
    let mut db = ConversationDb::open(&temp_db("s.db"), &KEY).unwrap();
    db.add_contact(&Contact {
        contact_id: [1; 16],
        contact_ref: [2; 16],
        display_name: "C".into(),
        card: vec![],
        verified: false,
        paired_at: 1,
        next_otk: 0,
        blocked: false,
    })
    .unwrap();
    let msg = MessageRow {
        message_id: [5; 16],
        contact_id: [1; 16],
        outgoing: true,
        body: "x".into(),
        created_at: 1,
        state: MessageState::Queued,
        envelope_id: Some([6; 16]),
        ack_secret: Some([7; 16]),
        expiry_min: Some(10),
        ..Default::default()
    };
    db.insert_message(&msg).unwrap();
    assert!(
        db.transition(&[5; 16], MessageState::AcceptedByRelay, 2)
            .unwrap()
    );
    assert!(
        db.transition(&[5; 16], MessageState::Acknowledged, 3)
            .unwrap()
    );
    assert!(
        !db.transition(&[5; 16], MessageState::DeliveryUnknown, 4)
            .unwrap()
    );
    assert!(
        !db.transition(&[5; 16], MessageState::AcceptedByRelay, 5)
            .unwrap()
    );
    assert_eq!(
        db.conversation(&[1; 16], 10).unwrap()[0].state,
        MessageState::Acknowledged
    );
    assert_eq!(db.message_by_envelope(&[6; 16]).unwrap(), Some([5; 16]));
    db.delete_message(&[5; 16], 6).unwrap();
    assert!(db.conversation(&[1; 16], 10).unwrap().is_empty());
}

#[test]
fn identity_and_sessions_round_trip() {
    let mut db = ConversationDb::open(&temp_db("i.db"), &KEY).unwrap();
    assert!(db.identity().unwrap().is_none());
    db.save_identity("pickled-account", &[9; 32], 1).unwrap();
    assert_eq!(
        db.identity().unwrap(),
        Some(("pickled-account".to_string(), [9; 32]))
    );
    db.add_contact(&Contact {
        contact_id: [1; 16],
        contact_ref: [2; 16],
        display_name: "C".into(),
        card: vec![],
        verified: false,
        paired_at: 1,
        next_otk: 0,
        blocked: false,
    })
    .unwrap();
    db.save_session(&[1; 16], "s1", "p1", 10).unwrap();
    db.save_session(&[1; 16], "s2", "p2", 20).unwrap();
    assert_eq!(db.sessions(&[1; 16]).unwrap()[0].0, "s2");
}

#[test]
fn schema_from_newer_app_is_refused() {
    let path = temp_db("v.db");
    drop(RelayDb::open(&path, &KEY).unwrap());
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        let hex: String = KEY.iter().map(|b| format!("{b:02x}")).collect();
        conn.execute_batch(&format!(
            "PRAGMA key = \"x'{hex}'\"; PRAGMA user_version = 99;"
        ))
        .unwrap();
    }
    assert!(matches!(
        RelayDb::open(&path, &KEY),
        Err(DbError::TooNew(99))
    ));
}
