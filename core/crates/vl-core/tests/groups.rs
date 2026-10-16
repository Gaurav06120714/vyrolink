//! Small groups via pairwise fan-out (ADR-024), end to end over real links.
//! Only the radio is simulated.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

mod common;
use common::*;
use vl_core::*;

const MTU: usize = 185;

struct Trio {
    a: Device,
    b: Device,
    c: Device,
    /// contact ids: x_y = y as seen by x
    a_b: [u8; 16],
    a_c: [u8; 16],
    b_a: [u8; 16],
}

fn trio(tag: &str, bob_knows_carol: bool) -> Trio {
    let (mut a, mut b, mut c) = (
        device(&format!("{tag}-a")),
        device(&format!("{tag}-b")),
        device(&format!("{tag}-c")),
    );
    let (a_b, b_a) = pair(&mut a, "Alice", &mut b, "Bob");
    let (a_c, _) = pair(&mut a, "Alice", &mut c, "Carol");
    if bob_knows_carol {
        pair(&mut b, "Bob", &mut c, "Carol");
    }
    Trio {
        a,
        b,
        c,
        a_b,
        a_c,
        b_a,
    }
}

fn bodies(d: &Device, g: &[u8; 16]) -> Vec<String> {
    let mut v: Vec<String> = d
        .node
        .group_messages(g, 50)
        .unwrap()
        .into_iter()
        .map(|m| m.body)
        .collect();
    v.sort();
    v
}

fn group(d: &Device, g: &[u8; 16]) -> GroupRow {
    d.node
        .groups()
        .unwrap()
        .into_iter()
        .find(|x| &x.group_id == g)
        .unwrap()
}

#[test]
fn create_send_and_count_deliveries() {
    let mut t = trio("g1", true);
    let g =
        t.a.node
            .create_group("Hike", "Alice", &[t.a_b, t.a_c], T0)
            .unwrap();
    encounter(&mut t.a, &mut t.b, T0 + 10, MTU);
    encounter(&mut t.a, &mut t.c, T0 + 20, MTU);
    assert_eq!(group(&t.b, &g).members.len(), 3);
    assert_eq!(group(&t.c, &g).epoch, 1);

    let m =
        t.b.node
            .send_group_message(&g, "trailhead at 7", None, T0 + 30)
            .unwrap();
    encounter(&mut t.b, &mut t.a, T0 + 40, MTU);
    encounter(&mut t.b, &mut t.c, T0 + 50, MTU);
    assert_eq!(bodies(&t.a, &g), vec!["trailhead at 7"]);
    assert_eq!(bodies(&t.c, &g), vec!["trailhead at 7"]);
    let at_a = &t.a.node.group_messages(&g, 5).unwrap()[0];
    assert_eq!(
        at_a.group_msg_id, m,
        "one logical id across pairwise copies"
    );
    assert_eq!(t.b.node.group_delivery(&m).unwrap(), (2, 2));
}

#[test]
fn message_before_invite_is_held_then_accepted() {
    let mut t = trio("g2", true);
    let g =
        t.a.node
            .create_group("Hike", "Alice", &[t.a_b, t.a_c], T0)
            .unwrap();
    encounter(&mut t.a, &mut t.b, T0 + 10, MTU);
    t.b.node
        .send_group_message(&g, "early bird", None, T0 + 20)
        .unwrap();
    // Bob relays Alice's invite for Carol and offers his own message first
    // (own envelopes lead the SUMMARY), so Carol receives the group message
    // before the roster in the same session: it is held, then accepted.
    encounter(&mut t.b, &mut t.c, T0 + 30, MTU);
    assert_eq!(group(&t.c, &g).epoch, 1, "invite relayed by Bob");
    assert_eq!(bodies(&t.c, &g), vec!["early bird"]);
}

#[test]
fn only_the_creator_changes_members_and_only_members_post() {
    let mut t = trio("g3", true);
    let mut mallory = device("g3-m");
    let (b_m, m_b) = pair(&mut t.b, "Bob", &mut mallory, "Mallory");
    let g =
        t.a.node
            .create_group("Hike", "Alice", &[t.a_b, t.a_c], T0)
            .unwrap();
    encounter(&mut t.a, &mut t.b, T0 + 10, MTU);

    let forged_state = Content::GroupState {
        group_id: g,
        epoch: 9,
        name: "Owned".into(),
        members: vec![GroupMember {
            fingerprint: [0xEE; 32],
            name: "x".into(),
        }],
    }
    .encode()
    .unwrap();
    mallory
        .node
        .send_raw_content(&m_b, &forged_state, T0 + 20)
        .unwrap();
    let forged_text = Content::GroupText {
        group_id: g,
        group_msg_id: [0x99; 16],
        epoch: 1,
        reply_to: None,
        text: "spam".into(),
    }
    .encode()
    .unwrap();
    mallory
        .node
        .send_raw_content(&m_b, &forged_text, T0 + 21)
        .unwrap();
    encounter(&mut mallory, &mut t.b, T0 + 30, MTU);
    let at_b = group(&t.b, &g);
    assert_eq!(
        (at_b.epoch, at_b.name.as_str()),
        (1, "Hike"),
        "non-creator state ignored"
    );
    assert!(bodies(&t.b, &g).is_empty(), "non-member message dropped");
    assert!(matches!(
        t.b.node
            .set_group_members(&g, "Bob", &[t.b_a, b_m], T0 + 40),
        Err(CoreError::NotGroupCreator)
    ));
}

#[test]
fn stale_roster_replay_is_ignored_and_removal_is_enforced() {
    let mut t = trio("g4", true);
    let g =
        t.a.node
            .create_group("Hike", "Alice", &[t.a_b, t.a_c], T0)
            .unwrap();
    encounter(&mut t.a, &mut t.b, T0 + 10, MTU);
    encounter(&mut t.a, &mut t.c, T0 + 20, MTU);

    t.a.node
        .set_group_members(&g, "Alice", &[t.a_b], T0 + 30)
        .unwrap();
    encounter(&mut t.a, &mut t.b, T0 + 40, MTU);
    encounter(&mut t.a, &mut t.c, T0 + 50, MTU);
    assert_eq!(group(&t.b, &g).epoch, 2);
    assert!(group(&t.c, &g).left, "Carol learns she was removed");
    assert!(matches!(
        t.c.node
            .send_group_message(&g, "still here?", None, T0 + 60),
        Err(CoreError::NotAllowed)
    ));

    let old = group(&t.a, &g);
    let mut members: Vec<GroupMember> = old
        .members
        .iter()
        .map(|(fp, n)| GroupMember {
            fingerprint: *fp,
            name: n.clone(),
        })
        .collect();
    members.push(GroupMember {
        fingerprint: [0x55; 32],
        name: "ghost".into(),
    });
    let stale = Content::GroupState {
        group_id: g,
        epoch: 1,
        name: old.name.clone(),
        members,
    };
    t.a.node
        .send_raw_content(&t.a_b, &stale.encode().unwrap(), T0 + 70)
        .unwrap();
    encounter(&mut t.a, &mut t.b, T0 + 80, MTU);
    let at_b = group(&t.b, &g);
    assert_eq!(
        (at_b.epoch, at_b.members.len()),
        (2, 2),
        "older epoch ignored"
    );
}

/// Bob's contact id as seen by Carol.
fn bob_in_carol(t: &Trio) -> [u8; 16] {
    t.c.node
        .contacts()
        .unwrap()
        .into_iter()
        .find(|c| c.display_name == "Bob")
        .map(|c| c.contact_id)
        .unwrap()
}

#[test]
fn removed_member_cannot_post_to_remaining_members() {
    let mut t = trio("g5", true);
    let g =
        t.a.node
            .create_group("Hike", "Alice", &[t.a_b, t.a_c], T0)
            .unwrap();
    encounter(&mut t.a, &mut t.b, T0 + 10, MTU);
    encounter(&mut t.a, &mut t.c, T0 + 20, MTU);
    t.a.node
        .set_group_members(&g, "Alice", &[t.a_b], T0 + 30)
        .unwrap();
    encounter(&mut t.a, &mut t.b, T0 + 40, MTU);

    let late = Content::GroupText {
        group_id: g,
        group_msg_id: [0x42; 16],
        epoch: 1,
        reply_to: None,
        text: "sent after removal".into(),
    };
    let to_bob = bob_in_carol(&t);
    t.c.node
        .send_raw_content(&to_bob, &late.encode().unwrap(), T0 + 50)
        .unwrap();
    encounter(&mut t.c, &mut t.b, T0 + 60, MTU);
    assert!(
        bodies(&t.b, &g).is_empty(),
        "no longer a member at Bob's epoch"
    );
}

#[test]
fn leaving_asks_the_creator_to_remove_me() {
    let mut t = trio("g6", true);
    let g =
        t.a.node
            .create_group("Hike", "Alice", &[t.a_b, t.a_c], T0)
            .unwrap();
    encounter(&mut t.a, &mut t.b, T0 + 10, MTU);
    encounter(&mut t.a, &mut t.c, T0 + 20, MTU);
    t.b.node.leave_group(&g, T0 + 30).unwrap();
    assert!(group(&t.b, &g).left);
    encounter(&mut t.b, &mut t.a, T0 + 40, MTU);
    let at_a = group(&t.a, &g);
    assert_eq!((at_a.epoch, at_a.members.len()), (2, 2));
    encounter(&mut t.a, &mut t.c, T0 + 50, MTU);
    assert_eq!(
        group(&t.c, &g).members.len(),
        2,
        "remaining members updated"
    );
}

#[test]
fn unpaired_members_are_not_reached_and_not_counted() {
    let mut t = trio("g7", false);
    let g =
        t.a.node
            .create_group("Hike", "Alice", &[t.a_b, t.a_c], T0)
            .unwrap();
    encounter(&mut t.a, &mut t.b, T0 + 10, MTU);
    let m =
        t.b.node
            .send_group_message(&g, "only Alice can get this", None, T0 + 20)
            .unwrap();
    assert_eq!(t.b.node.group_delivery(&m).unwrap(), (0, 1));
    encounter(&mut t.b, &mut t.a, T0 + 30, MTU);
    assert_eq!(t.b.node.group_delivery(&m).unwrap(), (1, 1));
    assert!(matches!(
        t.a.node.create_group("Too big", "Alice", &[], T0),
        Err(CoreError::InvalidGroupSize)
    ));
}
