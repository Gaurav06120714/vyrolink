//! QR "paper" messages (ADR-036): the same sealed envelope, carried by a camera.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

mod common;
use common::*;
use vl_core::*;
use vl_store::MessageState;

const MTU: usize = 185;

#[test]
fn a_scanned_code_delivers_once_and_the_receipt_follows_by_bluetooth() {
    let (mut a, mut b) = (device("pp-a"), device("pp-b"));
    let (b_in_a, a_in_b) = pair(&mut a, "A", &mut b, "B");
    let m = a
        .node
        .send_text(&b_in_a, "the gate code is 4471", T0)
        .unwrap();
    let code = a.node.paper_message(&m).unwrap();
    assert!(code.starts_with("VLM1:"));
    assert!(
        !code.contains("4471"),
        "the code is the sealed envelope, not text"
    );

    assert_eq!(b.node.receive_paper(&code, T0 + 30).unwrap(), a_in_b);
    assert_eq!(
        b.node.receive_paper(&code, T0 + 31).unwrap(),
        a_in_b,
        "scanning twice is harmless"
    );
    let got = b.node.conversation(&a_in_b, 10).unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].body, "the gate code is 4471");
    assert_ne!(
        state(&a, &b_in_a, &m),
        MessageState::Acknowledged,
        "no proof yet"
    );

    // The phones meet later: the receipt comes back, nothing is duplicated.
    encounter(&mut b, &mut a, T0 + 120, MTU);
    assert_eq!(state(&a, &b_in_a, &m), MessageState::Acknowledged);
    assert_eq!(b.node.conversation(&a_in_b, 10).unwrap().len(), 1);
}

#[test]
fn wrong_phone_tampered_or_expired_codes_are_refused() {
    let (mut a, mut b, mut c) = (device("pq-a"), device("pq-b"), device("pq-c"));
    let (b_in_a, a_in_b) = pair(&mut a, "A", &mut b, "B");
    pair(&mut a, "A", &mut c, "C");
    let m = a.node.send_text(&b_in_a, "only for B", T0).unwrap();
    let code = a.node.paper_message(&m).unwrap();

    assert!(matches!(
        c.node.receive_paper(&code, T0 + 10),
        Err(CoreError::NotForThisPhone)
    ));
    assert!(c.node.search("only", None, 5).unwrap().is_empty());

    // Change one Base45 character inside the ciphertext.
    let mut chars: Vec<char> = code.chars().collect();
    let i = chars.len() - 20;
    chars[i] = if chars[i] == 'A' { 'B' } else { 'A' };
    let tampered: String = chars.into_iter().collect();
    assert!(b.node.receive_paper(&tampered, T0 + 10).is_err());
    assert!(b.node.receive_paper("VLM1:not a code", T0 + 10).is_err());
    assert!(b.node.conversation(&a_in_b, 10).unwrap().is_empty());

    let late = T0 + 8 * 24 * 3600;
    assert!(matches!(
        b.node.receive_paper(&code, late),
        Err(CoreError::NotForThisPhone)
    ));
    // The untampered code still works in time.
    b.node.receive_paper(&code, T0 + 20).unwrap();
    assert_eq!(b.node.conversation(&a_in_b, 10).unwrap().len(), 1);
}
