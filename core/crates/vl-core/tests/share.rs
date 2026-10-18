//! Shared locations (ADR-034) and polls (ADR-035) between complete nodes.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

mod common;
use common::*;
use vl_core::*;

const MTU: usize = 185;

fn opts(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn a_shared_location_arrives_exactly() {
    let (mut a, mut b) = (device("loc-a"), device("loc-b"));
    let (b_in_a, a_in_b) = pair(&mut a, "A", &mut b, "B");
    let m = a
        .node
        .send_location(&b_in_a, 174_123_456, 785_654_321, 9, 1_790_000_000_000, T0)
        .unwrap();
    encounter(&mut a, &mut b, T0 + 60, MTU);
    let got = b.node.conversation(&a_in_b, 10).unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].body, LOCATION_BODY);
    let loc = b.node.location(&m).unwrap().unwrap();
    assert_eq!(
        (loc.lat_e7, loc.lon_e7, loc.accuracy_m),
        (174_123_456, 785_654_321, 9)
    );
    assert_eq!(loc.captured_ms, 1_790_000_000_000);
    // The sender keeps its own copy.
    assert!(a.node.location(&m).unwrap().is_some());
}

#[test]
fn chat_poll_votes_count_on_both_phones() {
    let (mut a, mut b) = (device("poll-a"), device("poll-b"));
    let (b_in_a, _) = pair(&mut a, "A", &mut b, "B");
    let p = a
        .node
        .send_poll(
            &b_in_a,
            "Where do we meet?",
            &opts(&["North gate", "Library\nhall"]),
            false,
            T0,
        )
        .unwrap();
    encounter(&mut a, &mut b, T0 + 10, MTU);
    let seen = b.node.poll(&p).unwrap().unwrap();
    assert_eq!(seen.question, "Where do we meet?");
    assert_eq!(
        seen.options,
        vec!["North gate", "Library hall"],
        "newlines are cleaned"
    );

    // Single choice: two options at once is refused; a valid vote counts.
    assert!(b.node.vote(&p, 0b11, T0 + 20).is_err());
    b.node.vote(&p, 0b10, T0 + 20).unwrap();
    a.node.vote(&p, 0b01, T0 + 20).unwrap();
    encounter(&mut a, &mut b, T0 + 30, MTU);
    for d in [&a, &b] {
        let v = d.node.poll(&p).unwrap().unwrap();
        assert_eq!(v.counts, vec![1, 1]);
        assert_eq!(v.voters, 2);
    }
    assert_eq!(a.node.poll(&p).unwrap().unwrap().mine, 0b01);

    // Changing a vote replaces it; 0 withdraws it.
    b.node.vote(&p, 0b01, T0 + 40).unwrap();
    encounter(&mut b, &mut a, T0 + 50, MTU);
    assert_eq!(a.node.poll(&p).unwrap().unwrap().counts, vec![2, 0]);
    b.node.vote(&p, 0, T0 + 60).unwrap();
    encounter(&mut b, &mut a, T0 + 70, MTU);
    assert_eq!(a.node.poll(&p).unwrap().unwrap().voters, 1);
}

#[test]
fn group_poll_counts_members_only() {
    let (mut a, mut b, mut c, mut d) = (
        device("gp-a"),
        device("gp-b"),
        device("gp-c"),
        device("gp-d"),
    );
    let (a_b, _) = pair(&mut a, "Alice", &mut b, "Bob");
    let (a_c, _) = pair(&mut a, "Alice", &mut c, "Carol");
    pair(&mut b, "Bob", &mut c, "Carol");
    let (_, d_in_b) = pair(&mut b, "Bob", &mut d, "Dan");
    let g = a
        .node
        .create_group("Hike", "Alice", &[a_b, a_c], T0)
        .unwrap();
    encounter(&mut a, &mut b, T0 + 10, MTU);
    encounter(&mut a, &mut c, T0 + 20, MTU);

    let p = a
        .node
        .send_group_poll(
            &g,
            "Start time?",
            &opts(&["7:00", "8:00", "9:00"]),
            true,
            T0 + 30,
        )
        .unwrap();
    encounter(&mut a, &mut b, T0 + 40, MTU);
    encounter(&mut a, &mut c, T0 + 50, MTU);
    assert_eq!(b.node.poll(&p).unwrap().unwrap().options.len(), 3);
    assert!(
        b.node
            .group_messages(&g, 5)
            .unwrap()
            .iter()
            .any(|m| m.group_msg_id == p)
    );

    // Multiple choice is allowed in this poll.
    b.node.vote(&p, 0b011, T0 + 60).unwrap();
    c.node.vote(&p, 0b010, T0 + 60).unwrap();
    encounter(&mut b, &mut a, T0 + 70, MTU);
    encounter(&mut c, &mut a, T0 + 80, MTU);
    let at_a = a.node.poll(&p).unwrap().unwrap();
    assert_eq!(at_a.counts, vec![1, 2, 0]);
    assert_eq!(at_a.voters, 2);

    // Dan is Bob's contact but not in the group: he has no poll to vote on,
    // and nothing he sends can count in it.
    assert!(d.node.vote(&p, 0b001, T0 + 90).is_err());
    let _ = d_in_b;
    assert_eq!(a.node.poll(&p).unwrap().unwrap().voters, 2);
}
