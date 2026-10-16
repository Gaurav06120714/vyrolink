//! Shared end-to-end helpers: complete nodes with real SQLCipher databases,
//! Noise links and Olm; only the radio is simulated.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, dead_code)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use vl_core::*;
use vl_routing::Policy;

pub const T0: u64 = 1_790_000_000;

pub struct Device {
    pub node: Node,
    pub dir: PathBuf,
}

pub fn device(name: &str) -> Device {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "vl-e2e-{}-{}-{name}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let relay_db = dir.join("relay.db");
    let conversation_db = dir.join("conv.db");
    let cfg = NodeConfig {
        relay_db: &relay_db,
        relay_key: [1; 32],
        conversation_db: &conversation_db,
        conversation_key: [2; 32],
        policy: Policy::default_spray(),
    };
    let mut node = Node::open(&cfg).unwrap();
    node.create_identity(T0).unwrap();
    Device { node, dir }
}

pub fn reopen(d: Device) -> Device {
    let relay_db = d.dir.join("relay.db");
    let conversation_db = d.dir.join("conv.db");
    let dir = d.dir.clone();
    drop(d.node);
    let cfg = NodeConfig {
        relay_db: &relay_db,
        relay_key: [1; 32],
        conversation_db: &conversation_db,
        conversation_key: [2; 32],
        policy: Policy::default_spray(),
    };
    Device {
        node: Node::open(&cfg).unwrap(),
        dir,
    }
}

pub fn pair(a: &mut Device, an: &str, b: &mut Device, bn: &str) -> ([u8; 16], [u8; 16]) {
    let ca = a.node.pairing_card(an, T0).unwrap();
    let cb = b.node.pairing_card(bn, T0).unwrap();
    let b_in_a = a.node.complete_pairing(&cb, T0).unwrap();
    let a_in_b = b.node.complete_pairing(&ca, T0).unwrap();
    assert_eq!(b_in_a.safety_number, a_in_b.safety_number);
    (b_in_a.contact_id, a_in_b.contact_id)
}

/// Connects two devices and exchanges bytes in `mtu`-sized writes until both go quiet.
pub fn encounter(a: &mut Device, b: &mut Device, now: u64, mtu: usize) {
    let (la, first) = a.node.link_open(true).unwrap();
    let (lb, _) = b.node.link_open(false).unwrap();
    let mut to_b = first;
    let mut to_a = Vec::new();
    for _ in 0..200 {
        if to_a.is_empty() && to_b.is_empty() {
            break;
        }
        let mut next_to_a = Vec::new();
        for chunk in to_b.chunks(mtu) {
            next_to_a.extend(b.node.link_receive(lb, chunk, now).unwrap());
        }
        let mut next_to_b = Vec::new();
        for chunk in to_a.chunks(mtu) {
            next_to_b.extend(a.node.link_receive(la, chunk, now).unwrap());
        }
        to_a = next_to_a;
        to_b = next_to_b;
    }
    assert!(to_a.is_empty() && to_b.is_empty(), "link did not quiesce");
    a.node.link_close(la);
    b.node.link_close(lb);
}

pub fn state(d: &Device, contact: &[u8; 16], mid: &[u8; 16]) -> MessageState {
    d.node
        .conversation(contact, 100)
        .unwrap()
        .into_iter()
        .find(|m| &m.message_id == mid)
        .unwrap()
        .state
}
