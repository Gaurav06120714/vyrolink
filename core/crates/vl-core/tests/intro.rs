//! Introductions (ADR-037) between complete nodes over real link sessions.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

mod common;
use common::*;
use vl_core::*;

const MTU: usize = 185;

fn named(d: &Device, name: &str) -> Option<ContactInfo> {
    d.node
        .contacts()
        .unwrap()
        .into_iter()
        .find(|c| c.display_name == name)
}

/// The introduction offered to `d` in its chat with `from`.
fn offered(d: &Device, from: &[u8; 16]) -> [u8; 16] {
    d.node
        .conversation(from, 20)
        .unwrap()
        .iter()
        .find_map(|m| d.node.intro_for_message(&m.message_id).unwrap())
        .unwrap()
        .intro_id
}

fn meet(x: &mut Device, y: &mut Device, t: u64) {
    encounter(x, y, t, MTU);
    encounter(y, x, t + 1, MTU);
}

#[test]
fn introduced_contacts_can_talk_and_start_unverified() {
    let (mut a, mut b, mut c) = (device("in-a"), device("in-b"), device("in-c"));
    let (b_in_a, a_in_b) = pair(&mut a, "Asha", &mut b, "Bala");
    let (c_in_a, a_in_c) = pair(&mut a, "Asha", &mut c, "Chitra");
    a.node.introduce(&b_in_a, &c_in_a, T0).unwrap();
    meet(&mut a, &mut b, T0 + 10);
    meet(&mut a, &mut c, T0 + 20);

    let (ib, ic) = (offered(&b, &a_in_b), offered(&c, &a_in_c));
    assert_eq!(ib, ic, "one introduction, seen by both");
    assert!(
        named(&b, "Chitra").is_none(),
        "nothing happens until both accept"
    );
    b.node.accept_intro(&ib, "Bala", T0 + 30).unwrap();
    c.node.accept_intro(&ic, "Chitra", T0 + 30).unwrap();
    meet(&mut b, &mut a, T0 + 40);
    meet(&mut c, &mut a, T0 + 50);
    meet(&mut a, &mut b, T0 + 60);
    meet(&mut a, &mut c, T0 + 70);

    let chitra = named(&b, "Chitra").unwrap();
    let bala = named(&c, "Bala").unwrap();
    assert!(
        !chitra.verified && !bala.verified,
        "introductions are not in-person verification"
    );
    assert_eq!(chitra.introduced_by.as_deref(), Some("Asha"));
    assert_eq!(
        chitra.safety_number, bala.safety_number,
        "they can still compare in person later"
    );

    b.node
        .send_text(
            &chitra.contact_id,
            "hi Chitra, Asha told me about you",
            T0 + 80,
        )
        .unwrap();
    c.node
        .send_text(&bala.contact_id, "hi Bala", T0 + 80)
        .unwrap();
    meet(&mut b, &mut c, T0 + 90);
    let bodies = |d: &Device, id: &[u8; 16]| -> Vec<String> {
        d.node
            .conversation(id, 10)
            .unwrap()
            .into_iter()
            .map(|m| m.body)
            .collect()
    };
    assert!(bodies(&c, &bala.contact_id).contains(&"hi Chitra, Asha told me about you".to_owned()));
    assert!(bodies(&b, &chitra.contact_id).contains(&"hi Bala".to_owned()));
}

#[test]
fn a_declined_introduction_connects_nobody() {
    let (mut a, mut b, mut c) = (device("id-a"), device("id-b"), device("id-c"));
    let (b_in_a, a_in_b) = pair(&mut a, "Asha", &mut b, "Bala");
    let (c_in_a, a_in_c) = pair(&mut a, "Asha", &mut c, "Chitra");
    a.node.introduce(&b_in_a, &c_in_a, T0).unwrap();
    meet(&mut a, &mut b, T0 + 10);
    meet(&mut a, &mut c, T0 + 20);
    b.node
        .accept_intro(&offered(&b, &a_in_b), "Bala", T0 + 30)
        .unwrap();
    let ic = offered(&c, &a_in_c);
    c.node.decline_intro(&ic).unwrap();
    assert!(
        c.node.accept_intro(&ic, "Chitra", T0 + 31).is_err(),
        "declined stays declined"
    );
    for t in [T0 + 40, T0 + 60] {
        meet(&mut b, &mut a, t);
        meet(&mut c, &mut a, t + 10);
    }
    assert!(named(&b, "Chitra").is_none());
    assert!(named(&c, "Bala").is_none());
}

#[test]
fn introducing_people_who_already_know_each_other_changes_nothing() {
    let (mut a, mut b, mut c) = (device("ik-a"), device("ik-b"), device("ik-c"));
    let (b_in_a, a_in_b) = pair(&mut a, "Asha", &mut b, "Bala");
    let (c_in_a, _) = pair(&mut a, "Asha", &mut c, "Chitra");
    pair(&mut b, "Bala", &mut c, "Chitra");
    assert!(
        a.node.introduce(&b_in_a, &b_in_a, T0).is_err(),
        "not to themselves"
    );
    a.node.introduce(&b_in_a, &c_in_a, T0).unwrap();
    meet(&mut a, &mut b, T0 + 10);
    let i = b
        .node
        .intro_for_message(&b.node.conversation(&a_in_b, 10).unwrap()[0].message_id);
    assert_eq!(
        i.unwrap().unwrap().state,
        INTRO_CONNECTED,
        "already a contact"
    );
    assert_eq!(b.node.contacts().unwrap().len(), 2, "no duplicate");
}
