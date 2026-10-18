//! Pairing in person when a phone shows its code more than once (ADR-031).
//! Found on real phones on 2026-10-17: reopening "Add contact" between the
//! two scans makes a new card, and the phones used to derive different
//! pairwise keys, so each filed the other's mail as someone else's.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

mod common;
use common::*;

const MTU: usize = 185;

fn contact_named(d: &Device, name: &str) -> [u8; 16] {
    d.node
        .contacts()
        .unwrap()
        .into_iter()
        .find(|c| c.display_name == name)
        .unwrap()
        .contact_id
}

fn bodies(d: &Device, contact: &[u8; 16]) -> Vec<String> {
    d.node
        .conversation(contact, 50)
        .unwrap()
        .into_iter()
        .map(|m| m.body)
        .collect()
}

/// Both directions deliver.
fn assert_talks(a: &mut Device, b: &mut Device) {
    let (b_in_a, a_in_b) = (contact_named(a, "B"), contact_named(b, "A"));
    a.node.send_text(&b_in_a, "hello from A", T0 + 10).unwrap();
    b.node.send_text(&a_in_b, "hello from B", T0 + 10).unwrap();
    encounter(a, b, T0 + 60, MTU);
    encounter(b, a, T0 + 120, MTU);
    assert!(
        bodies(b, &a_in_b).contains(&"hello from A".to_owned()),
        "B reads A"
    );
    assert!(
        bodies(a, &b_in_a).contains(&"hello from B".to_owned()),
        "A reads B"
    );
}

#[test]
fn scanner_reopens_its_code_before_the_other_scans_it() {
    let (mut a, mut b) = (device("pc1-a"), device("pc1-b"));
    let _a1 = a.node.pairing_card("A", T0).unwrap();
    let b1 = b.node.pairing_card("B", T0).unwrap();
    a.node.complete_pairing(&b1, T0).unwrap();
    // A goes back and opens "Add contact" again: a new card A2 is shown.
    let a2 = a.node.pairing_card("A", T0 + 5).unwrap();
    b.node.complete_pairing(&a2, T0 + 5).unwrap();
    assert_talks(&mut a, &mut b);
}

#[test]
fn shower_reopens_its_code_after_being_scanned() {
    let (mut a, mut b) = (device("pc2-a"), device("pc2-b"));
    let a1 = a.node.pairing_card("A", T0).unwrap();
    let b1 = b.node.pairing_card("B", T0).unwrap();
    a.node.complete_pairing(&b1, T0).unwrap();
    // B reopens its screen (new card B2), then scans A's first card.
    let _b2 = b.node.pairing_card("B", T0 + 5).unwrap();
    b.node.complete_pairing(&a1, T0 + 5).unwrap();
    assert_talks(&mut a, &mut b);
}

#[test]
fn several_reopenings_on_both_sides() {
    let (mut a, mut b) = (device("pc3-a"), device("pc3-b"));
    let _ = a.node.pairing_card("A", T0).unwrap();
    let b1 = b.node.pairing_card("B", T0).unwrap();
    let _ = b.node.pairing_card("B", T0 + 1).unwrap();
    let _ = a.node.pairing_card("A", T0 + 2).unwrap();
    a.node.complete_pairing(&b1, T0 + 3).unwrap();
    let a3 = a.node.pairing_card("A", T0 + 4).unwrap();
    let _ = b.node.pairing_card("B", T0 + 5).unwrap();
    b.node.complete_pairing(&a3, T0 + 6).unwrap();
    assert_talks(&mut a, &mut b);
}
