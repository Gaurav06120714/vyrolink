//! Reproducible routing-store benchmark on a real SQLCipher RelayDb.
//! `cargo run --release -p vl-store --example bench_relay`
//! Reports the median of 5 runs per case, in milliseconds.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use std::time::Instant;
use vl_crypto::ack_commitment;
use vl_proto::*;
use vl_routing::{LinkSession, Policy, Store, Stored};
use vl_store::RelayDb;

const NOW: u32 = 29_000_000;

fn id(i: u32, kind: u8) -> [u8; 16] {
    let mut b = [kind; 16];
    b[..4].copy_from_slice(&i.to_be_bytes());
    b
}

fn envelope(i: u32) -> Envelope {
    Envelope {
        header: Header {
            kind: EnvelopeKind::Data,
            flags: FLAG_VACCINE,
            hop_count: 1,
            hop_limit: 8,
            copy_budget: 4,
            envelope_id: id(i, 1),
            recipient_tag: id(i % 400, 2),
            expiry_min: NOW + 1000,
            ack_commitment: ack_commitment(&id(i, 3)),
            body_len: 1024,
        },
        body: vec![0x5A; 1024],
    }
}

fn fresh(n: u32) -> (RelayDb, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("vl-bench-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("r.db");
    let mut db = RelayDb::open(&path, &[7; 32]).unwrap();
    for i in 0..n {
        db.insert(Stored {
            envelope: envelope(i),
            own: i % 10 == 0,
            stored_at_min: NOW,
            retried: false,
        })
        .unwrap();
    }
    (db, dir)
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

fn main() {
    println!(
        "| stored envelopes | VACCINE(128 items, 64 match) ms | HELLO→SUMMARY ms | REQUEST(32) ms |"
    );
    println!("|---|---|---|---|");
    for n in [500u32, 2000, 5000] {
        let (mut vac, mut hello, mut req) = (Vec::new(), Vec::new(), Vec::new());
        for run in 0..5u32 {
            let (mut db, dir) = fresh(n);
            let policy = Policy::default_spray();
            let items: Vec<VaccineItem> = (0..128u32)
                .map(|k| VaccineItem {
                    ack_secret: if k % 2 == 0 {
                        id((k * 7 + run) % n, 3)
                    } else {
                        id(1_000_000 + k, 3)
                    },
                    expiry_min: NOW + 500,
                })
                .collect();
            let mut s = LinkSession::new(policy);
            let t = Instant::now();
            let _ = s.handle(LinkMessage::Vaccine(items), &mut db, NOW);
            vac.push(t.elapsed().as_secs_f64() * 1e3);

            let mut s = LinkSession::new(policy);
            let peer = s.open(&db).unwrap().remove(0);
            let t = Instant::now();
            let _ = s.handle(peer, &mut db, NOW);
            hello.push(t.elapsed().as_secs_f64() * 1e3);

            let items: Vec<RequestItem> = (0..32u32)
                .map(|k| RequestItem {
                    envelope_id: id(k * 13 % n, 1),
                    role: RequestRole::Carry,
                })
                .collect();
            let t = Instant::now();
            let _ = s.handle(LinkMessage::Request(items), &mut db, NOW);
            req.push(t.elapsed().as_secs_f64() * 1e3);
            drop(db);
            let _ = std::fs::remove_dir_all(dir);
        }
        println!(
            "| {n} | {:.1} | {:.1} | {:.1} |",
            median(vac),
            median(hello),
            median(req)
        );
    }
}
