//! Same ID after uninstall and reinstall (ADR-038): only the long-term keys
//! come back, and contacts pair again once.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

mod common;
use common::*;

const MTU: usize = 185;

fn bodies(d: &Device, contact: &[u8; 16]) -> Vec<String> {
    d.node
        .conversation(contact, 50)
        .unwrap()
        .into_iter()
        .map(|m| m.body)
        .collect()
}

#[test]
fn a_reinstalled_phone_keeps_its_id_and_talks_again_after_one_re_pair() {
    let (mut a, mut b) = (device("rs-a"), device("rs-b"));
    let (_, a_in_b) = pair(&mut a, "Asha", &mut b, "Bala");
    b.node.set_verified(&a_in_b, true).unwrap();
    let (id, safety) = (
        a.node.my_id_code().unwrap(),
        b.node.contacts().unwrap()[0].safety_number.clone(),
    );
    let backup = a.node.identity_backup().unwrap();

    // Asha uninstalls; Bala writes to the old install meanwhile.
    drop(a);
    b.node
        .send_text(&a_in_b, "sent while you were gone", T0 + 10)
        .unwrap();

    let mut a = blank_device("rs-a2");
    assert!(!a.node.has_identity());
    a.node.restore_identity(&backup, T0 + 20).unwrap();
    assert_eq!(a.node.my_id_code().unwrap(), id, "same ID after reinstall");
    assert!(
        a.node.contacts().unwrap().is_empty(),
        "contacts are not in the backup"
    );

    // Mail sealed to the lost install cannot be read, and must not break links.
    encounter(&mut b, &mut a, T0 + 30, MTU);
    encounter(&mut a, &mut b, T0 + 31, MTU);

    // One re-pair: Bala's phone recognises the same person.
    let (b_in_a, a_in_b2) = pair(&mut a, "Asha", &mut b, "Bala");
    assert_eq!(a_in_b2, a_in_b, "no duplicate contact");
    let asha = b.node.contacts().unwrap();
    assert_eq!(asha.len(), 1);
    assert!(asha[0].verified, "same identity, so verification stays");
    assert_eq!(asha[0].safety_number, safety, "same safety number");

    a.node.send_text(&b_in_a, "back again", T0 + 40).unwrap();
    b.node.send_text(&a_in_b, "welcome back", T0 + 40).unwrap();
    encounter(&mut a, &mut b, T0 + 50, MTU);
    encounter(&mut b, &mut a, T0 + 60, MTU);
    assert!(bodies(&b, &a_in_b).contains(&"back again".to_owned()));
    assert!(bodies(&a, &b_in_a).contains(&"welcome back".to_owned()));
}

#[test]
fn restore_is_refused_over_an_identity_or_from_damaged_bytes() {
    let a = device("rr-a");
    let backup = a.node.identity_backup().unwrap();
    let mut other = device("rr-b");
    assert!(
        other.node.restore_identity(&backup, T0).is_err(),
        "never replaces an identity"
    );
    let mut blank = blank_device("rr-c");
    assert!(blank.node.restore_identity(&backup[..40], T0).is_err());
    assert!(!blank.node.has_identity());
    blank.node.restore_identity(&backup, T0).unwrap();
    assert!(blank.node.has_identity());
    // The restored identity persists across a restart.
    let id = blank.node.my_id_code().unwrap();
    let blank = reopen(blank);
    assert_eq!(blank.node.my_id_code().unwrap(), id);
}
