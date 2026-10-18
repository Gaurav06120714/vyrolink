//! The internet mailbox (ADR-039) between complete nodes. The mailbox here is
//! a plain list filtered by recipient tag, which is all the real one does.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

mod common;
use common::*;
use vl_core::*;
use vl_proto::Envelope;

const MTU: usize = 185;

#[derive(Default)]
struct Mailbox(Vec<Vec<u8>>);

impl Mailbox {
    fn sync(&mut self, d: &mut Device, now: u64) {
        let up = d.node.internet_outbox(now).unwrap();
        for w in &up {
            let id = Envelope::decode(w).unwrap().header.envelope_id;
            if !self
                .0
                .iter()
                .any(|x| Envelope::decode(x).unwrap().header.envelope_id == id)
            {
                self.0.push(w.clone());
            }
        }
        d.node.internet_uploaded(&up, now).unwrap();
        let tags = d.node.internet_tags().unwrap();
        let mine: Vec<Vec<u8>> = self
            .0
            .iter()
            .filter(|w| tags.contains(&Envelope::decode(w).unwrap().header.recipient_tag))
            .cloned()
            .collect();
        for w in mine {
            d.node.receive_internet(&w, now).unwrap();
        }
    }
}

fn bodies(d: &Device, c: &[u8; 16]) -> Vec<String> {
    d.node
        .conversation(c, 50)
        .unwrap()
        .into_iter()
        .map(|m| m.body)
        .collect()
}

#[test]
fn two_phones_that_never_meet_talk_through_the_mailbox_with_receipts() {
    let (mut a, mut b) = (device("in-a"), device("in-b"));
    let (b_in_a, a_in_b) = pair(&mut a, "A", &mut b, "B");
    let mut mb = Mailbox::default();
    let m = a.node.send_text(&b_in_a, "over the internet", T0).unwrap();

    mb.sync(&mut a, T0 + 1);
    assert_eq!(state(&a, &b_in_a, &m), MessageState::AcceptedByRelay);
    mb.sync(&mut b, T0 + 2);
    assert_eq!(bodies(&b, &a_in_b), ["over the internet"]);
    // B's receipt goes up on the same sync; A sees Delivered on its next one.
    mb.sync(&mut b, T0 + 3);
    mb.sync(&mut a, T0 + 4);
    assert_eq!(state(&a, &b_in_a, &m), MessageState::Acknowledged);

    // Fetching again changes nothing: no duplicate messages.
    mb.sync(&mut b, T0 + 5);
    assert_eq!(bodies(&b, &a_in_b).len(), 1);
    // Nothing is uploaded twice.
    assert!(a.node.internet_outbox(T0 + 6).unwrap().is_empty());
}

#[test]
fn a_phone_with_internet_carries_mail_for_one_without() {
    // A has no internet; C (a stranger to both) meets A over Bluetooth and has
    // internet; B is online far away.
    let (mut a, mut b, mut c) = (device("ic-a"), device("ic-b"), device("ic-c"));
    let (b_in_a, a_in_b) = pair(&mut a, "A", &mut b, "B");
    let mut mb = Mailbox::default();
    a.node.send_text(&b_in_a, "via a stranger", T0).unwrap();
    encounter(&mut a, &mut c, T0 + 10, MTU);
    mb.sync(&mut c, T0 + 20);
    mb.sync(&mut b, T0 + 30);
    assert_eq!(bodies(&b, &a_in_b), ["via a stranger"]);
}

#[test]
fn a_dishonest_mailbox_cannot_forge_or_redirect_mail() {
    let (mut a, mut b, mut eve) = (device("id-a"), device("id-b"), device("id-e"));
    let (b_in_a, a_in_b) = pair(&mut a, "A", &mut b, "B");
    a.node.send_text(&b_in_a, "secret", T0).unwrap();
    let wire = a.node.internet_outbox(T0).unwrap().remove(0);

    // Tampered bytes, garbage and mail for someone else are ignored quietly.
    let mut bad = wire.clone();
    let last = bad.len() - 1;
    bad[last] ^= 1;
    assert!(!b.node.receive_internet(&bad, T0 + 1).unwrap());
    assert!(!b.node.receive_internet(b"garbage", T0 + 1).unwrap());
    assert!(!eve.node.receive_internet(&wire, T0 + 1).unwrap());
    // The genuine copy still gets through after the tampered one.
    assert!(b.node.receive_internet(&wire, T0 + 2).unwrap());
    assert_eq!(bodies(&b, &a_in_b), ["secret"]);
}
