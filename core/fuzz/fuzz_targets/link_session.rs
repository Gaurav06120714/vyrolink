#![no_main]
//! Feeds an adversarial sequence of decodable link messages into the routing
//! engine and checks its resource bounds hold.
use libfuzzer_sys::fuzz_target;
use vl_proto::{LinkMessage, MAX_COPY_BUDGET, MAX_FRAME_LEN};
use vl_routing::{LinkSession, MemStore, Policy, Store};

fuzz_target!(|data: &[u8]| {
    let policy = Policy::default_spray();
    let mut store = MemStore::with_tags([[0xC; 16]]);
    let mut session = LinkSession::new(policy);
    let mut rest = data;
    let mut now = 29_000_000u32;
    while rest.len() > 2 {
        let len = usize::from(u16::from_be_bytes([rest[0], rest[1]])).min(MAX_FRAME_LEN);
        let take = len.min(rest.len() - 2);
        let (msg, tail) = rest[2..].split_at(take);
        rest = tail;
        now = now.wrapping_add(u32::from(msg.first().copied().unwrap_or(0)));
        if let Ok(m) = LinkMessage::decode(msg) {
            let (replies, _) = session.handle(m, &mut store, now);
            for r in replies {
                assert!(r.encode().map(|b| b.len() <= MAX_FRAME_LEN).unwrap_or(false));
            }
        }
    }
    let all = store.all().unwrap();
    assert!(all.iter().filter(|s| !s.own).count() <= policy.session_accept_envelopes);
    assert!(store.carried_bytes().unwrap() <= policy.carry_cap_bytes);
    assert!(all.iter().all(|s| s.envelope.header.copy_budget <= MAX_COPY_BUDGET));
});
