//! Conversation-DB benchmark on a real SQLCipher file (median of 5 runs).
//! `cargo run --release -p vl-store --example bench_conversation`

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use std::time::Instant;
use vl_store::{Contact, ConversationDb, EventMeta, MessageRow, MessageState};

const WORDS: [&str; 12] = [
    "north", "gate", "meet", "tonight", "battery", "relay", "bridge", "river", "school", "market",
    "lunch", "station",
];

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

fn main() {
    let n: u32 = 10_000;
    let (mut insert, mut search, mut page, mut react, mut one) =
        (vec![], vec![], vec![], vec![], vec![]);
    for run in 0..5 {
        let dir = std::env::temp_dir().join(format!("vl-bench-conv-{}-{run}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut db = ConversationDb::open(&dir.join("c.db"), &[9; 32]).unwrap();
        for c in 0..10u8 {
            db.add_contact(&Contact {
                contact_id: [c; 16],
                contact_ref: [c + 100; 16],
                display_name: c.to_string(),
                ..Default::default()
            })
            .unwrap();
        }
        let t = Instant::now();
        for i in 0..n {
            let mut id = [0u8; 16];
            id[..4].copy_from_slice(&i.to_be_bytes());
            let body = format!(
                "{} {} {} message {i}",
                WORDS[i as usize % 12],
                WORDS[(i as usize / 12) % 12],
                WORDS[(i as usize / 7) % 12]
            );
            db.insert_message(&MessageRow {
                message_id: id,
                contact_id: [(i % 10) as u8; 16],
                outgoing: i % 2 == 0,
                body,
                created_at: i64::from(i),
                state: MessageState::Received,
                sent_at_ms: i64::from(i) * 1000,
                ..Default::default()
            })
            .unwrap();
        }
        insert.push(t.elapsed().as_secs_f64() * 1e6 / f64::from(n));
        let t = Instant::now();
        let hits = db.search("north gat", None, 50).unwrap().len();
        search.push(t.elapsed().as_secs_f64() * 1e3);
        assert!(hits > 0);
        let t = Instant::now();
        let _ = db.conversation(&[3; 16], 50).unwrap();
        page.push(t.elapsed().as_secs_f64() * 1e3);
        let t = Instant::now();
        let _ = db
            .message(&{
                let mut id = [0u8; 16];
                id[..4].copy_from_slice(&5000u32.to_be_bytes());
                id
            })
            .unwrap();
        one.push(t.elapsed().as_secs_f64() * 1e3);
        let t = Instant::now();
        for i in 0..100u32 {
            let mut target = [0u8; 16];
            target[..4].copy_from_slice(&(i * 10 + 3).to_be_bytes());
            db.upsert_reaction(
                &EventMeta {
                    target,
                    contact_id: [3; 16],
                    from_me: true,
                    sent_at_ms: 1,
                    received_at: 1,
                },
                1,
                &[1; 16],
            )
            .unwrap();
        }
        react.push(t.elapsed().as_secs_f64() * 1e3 / 100.0);
        drop(db);
        let _ = std::fs::remove_dir_all(dir);
    }
    println!("| operation ({n} messages, 10 contacts) | median |");
    println!("|---|---|");
    println!(
        "| insert_message incl. FTS index + WAL fsync | {:.0} µs/message |",
        median(insert)
    );
    println!(
        "| search 2 prefix terms, top 50 | {:.2} ms |",
        median(search)
    );
    println!("| conversation page (50 newest) | {:.2} ms |", median(page));
    println!("| message by id | {:.3} ms |", median(one));
    println!("| upsert_reaction | {:.2} ms/event |", median(react));
}
