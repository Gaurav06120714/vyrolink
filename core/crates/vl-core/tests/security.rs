//! End-to-end encryption against a relay that is curious, compromised or
//! actively malicious (Doc 23). Phone A writes to phone C; phone B carries the
//! envelope. B is a legitimate contact of both, so it holds real keys for A
//! and for C, and it still must not read, alter, forge or suppress A→C mail.
//! `Wire` is a hand-driven link: exactly what an attacker's phone can send.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

mod common;
use common::*;
use vl_crypto::{LinkHandshake, LinkTransport};
use vl_proto::{
    ATTACHMENT_FILE, CustodyItem, CustodyStatus, Deframer, Envelope, Hello, LINK_VERSION,
    LinkMessage, MAX_FRAME_LEN, RelayClass, frame,
};
use vl_store::MessageState;

const MTU: usize = 185;
const CANARY: &str = "CANARY-7f3a meet at the north gate at six";

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

/// Every byte the device keeps: its database files (SQLCipher pages, WAL).
fn disk_bytes(d: &Device) -> Vec<u8> {
    let mut all = Vec::new();
    for e in std::fs::read_dir(&d.dir).unwrap().flatten() {
        all.extend(std::fs::read(e.path()).unwrap_or_default());
    }
    all
}

/// Envelopes in the device's relay queue, encoded as they would go on the air.
fn queue(d: &Device) -> Vec<(bool, Envelope)> {
    d.node
        .relay_store()
        .all()
        .unwrap()
        .into_iter()
        .map(|s| (s.own, s.envelope))
        .collect()
}

fn own_envelope(d: &Device) -> Envelope {
    let mine: Vec<_> = queue(d).into_iter().filter(|(own, _)| *own).collect();
    assert_eq!(mine.len(), 1, "exactly one queued envelope expected");
    mine[0].1.clone()
}

fn bodies(d: &Device, contact: &[u8; 16]) -> Vec<String> {
    d.node
        .conversation(contact, 100)
        .unwrap()
        .into_iter()
        .map(|m| m.body)
        .collect()
}

struct Wire {
    t: LinkTransport,
    deframer: Deframer,
    link: u64,
}

impl Wire {
    fn connect(d: &mut Device, now: u64) -> Self {
        let (link, _) = d.node.link_open(false).unwrap();
        let mut hs = LinkHandshake::initiator().unwrap();
        let first = frame(&hs.write_message().unwrap()).unwrap();
        let reply = d.node.link_receive(link, &first, now).unwrap();
        let mut deframer = Deframer::new(MAX_FRAME_LEN + 16);
        let frames = deframer.push(&reply).unwrap();
        hs.read_message(&frames[0]).unwrap();
        let mut t = hs.into_transport().unwrap();
        for f in &frames[1..] {
            t.decrypt(f).unwrap();
        }
        let mut w = Wire { t, deframer, link };
        let hello = LinkMessage::Hello(Hello {
            link_versions: vec![LINK_VERSION],
            max_frame: MAX_FRAME_LEN as u16,
            relay_class: RelayClass::BestEffort,
            relay_willing: true,
            free_capacity_bucket: 255,
            l2cap_psm: None,
            features: 0,
        });
        w.send(d, &hello, now);
        w
    }

    fn send(&mut self, d: &mut Device, m: &LinkMessage, now: u64) -> Vec<LinkMessage> {
        let bytes = frame(&self.t.encrypt(&m.encode().unwrap()).unwrap()).unwrap();
        let reply = d.node.link_receive(self.link, &bytes, now).unwrap();
        self.deframer
            .push(&reply)
            .unwrap()
            .iter()
            .map(|f| LinkMessage::decode(&self.t.decrypt(f).unwrap()).unwrap())
            .collect()
    }

    /// Pushes an envelope unasked and returns the victim's custody answer.
    fn push(&mut self, d: &mut Device, env: &Envelope, now: u64) -> CustodyStatus {
        let replies = self.send(d, &LinkMessage::Envelope(env.clone()), now);
        replies
            .iter()
            .find_map(|m| match m {
                LinkMessage::Custody(items) => items
                    .iter()
                    .find(|i| i.envelope_id == env.header.envelope_id)
                    .map(|i| i.status),
                _ => None,
            })
            .expect("custody reply")
    }
}

#[test]
fn a_to_c_through_b_only_c_can_read_and_b_holds_only_ciphertext() {
    let (mut a, mut b, mut c) = (device("sec1-a"), device("sec1-b"), device("sec1-c"));
    let (c_in_a, a_in_c) = pair(&mut a, "A", &mut c, "C");
    // B is a real contact of both, so it holds genuine keys for A and for C.
    let (b_in_a, a_in_b) = pair(&mut a, "A", &mut b, "B");
    let (_c_in_b, b_in_c) = pair(&mut b, "B", &mut c, "C");

    let mid = a.node.send_text(&c_in_a, CANARY, T0).unwrap();
    let file: Vec<u8> = b"FILE-CANARY "
        .iter()
        .copied()
        .cycle()
        .take(6_000)
        .collect();
    let att = a
        .node
        .send_attachment(&c_in_a, ATTACHMENT_FILE, "plan.txt", &file, "", None, T0)
        .unwrap();

    encounter(&mut a, &mut b, T0 + 60, MTU);
    assert_eq!(state(&a, &c_in_a, &mid), MessageState::AcceptedByRelay);

    // B now carries A→C mail. Restart B so everything is on disk.
    let b = reopen(b);
    let carried: Vec<_> = queue(&b).into_iter().filter(|(own, _)| !own).collect();
    assert!(carried.len() > 1, "B carries the text and the file parts");
    for (_, env) in &carried {
        let wire = env.encode();
        assert!(
            !contains(&wire, CANARY.as_bytes()),
            "queue holds no plaintext"
        );
        assert!(
            !contains(&wire, b"FILE-CANARY"),
            "queue holds no file bytes"
        );
    }
    let disk = disk_bytes(&b);
    assert!(!disk.is_empty());
    assert!(
        !contains(&disk, CANARY.as_bytes()),
        "B's disk holds no plaintext"
    );
    assert!(
        !contains(&disk, b"FILE-CANARY"),
        "B's disk holds no file bytes"
    );
    assert!(
        !contains(&disk, b"plan.txt"),
        "B's disk holds no attachment metadata"
    );
    // B could not open anything: nothing appeared in B's own conversations.
    assert!(bodies(&b, &a_in_b).is_empty());
    assert!(bodies(&b, &b_in_c).iter().all(|m| !m.contains("CANARY")));
    assert!(b.node.search("CANARY", None, 10).unwrap().is_empty());
    assert!(b.node.attachment(&att).unwrap().is_none());

    let mut b = b;
    encounter(&mut b, &mut c, T0 + 3600, MTU);
    assert!(
        bodies(&c, &a_in_c).contains(&CANARY.to_owned()),
        "C reads it"
    );
    assert_eq!(
        c.node.attachment_data(&att).unwrap().unwrap(),
        file,
        "C gets the file"
    );

    // The proof of delivery travels back through B; only then is A "Delivered".
    encounter(&mut c, &mut b, T0 + 3700, MTU);
    encounter(&mut b, &mut a, T0 + 3800, MTU);
    assert_eq!(state(&a, &c_in_a, &mid), MessageState::Acknowledged);
    assert!(
        !contains(&disk_bytes(&b), CANARY.as_bytes()),
        "still nothing readable on B afterwards"
    );
    let _ = b_in_a;
}

#[test]
fn tampered_copy_is_rejected_and_cannot_block_the_genuine_one() {
    let (mut a, mut c, mut m) = (device("sec2-a"), device("sec2-c"), device("sec2-m"));
    let (c_in_a, a_in_c) = pair(&mut a, "A", &mut c, "C");
    a.node.send_text(&c_in_a, CANARY, T0).unwrap();
    let genuine = own_envelope(&a);
    let _ = &mut m;

    let mut wire = Wire::connect(&mut c, T0 + 60);
    // Flip one ciphertext byte: the AEAD tag fails, nothing is shown.
    let mut body = genuine.clone();
    body.body[40] ^= 0x01;
    wire.push(&mut c, &body, T0 + 60);
    // Change an authenticated header field (expiry): also rejected.
    let mut header = genuine.clone();
    header.header.expiry_min -= 1;
    wire.push(&mut c, &header, T0 + 60);
    // Truncated or padded bodies never even parse as a valid envelope.
    let mut short = genuine.clone();
    short.body.truncate(short.body.len() - 1);
    assert!(Envelope::decode(&short.encode()).is_err());
    assert!(
        bodies(&c, &a_in_c).is_empty(),
        "no tampered message accepted"
    );

    // The bad copies did not use up the envelope id: the genuine one arrives.
    assert_eq!(
        wire.push(&mut c, &genuine, T0 + 60),
        CustodyStatus::DeliveredLocal
    );
    assert_eq!(bodies(&c, &a_in_c), vec![CANARY.to_owned()]);
}

#[test]
fn replays_and_duplicates_are_shown_once_even_after_restart() {
    let (mut a, mut c) = (device("sec3-a"), device("sec3-c"));
    let (c_in_a, a_in_c) = pair(&mut a, "A", &mut c, "C");
    a.node.send_text(&c_in_a, CANARY, T0).unwrap();
    let genuine = own_envelope(&a);

    let mut wire = Wire::connect(&mut c, T0 + 60);
    assert_eq!(
        wire.push(&mut c, &genuine, T0 + 60),
        CustodyStatus::DeliveredLocal
    );
    assert_eq!(
        wire.push(&mut c, &genuine, T0 + 61),
        CustodyStatus::Duplicate
    );
    // Relays change only the unauthenticated hop fields; still the same envelope.
    let mut hopped = genuine.clone();
    hopped.header.hop_count += 1;
    hopped.header.copy_budget = 1;
    assert_eq!(
        wire.push(&mut c, &hopped, T0 + 62),
        CustodyStatus::Duplicate
    );

    let mut c = reopen(c);
    let mut wire = Wire::connect(&mut c, T0 + 120);
    assert_eq!(
        wire.push(&mut c, &genuine, T0 + 120),
        CustodyStatus::Duplicate
    );
    assert_eq!(bodies(&c, &a_in_c), vec![CANARY.to_owned()]);
}

#[test]
fn out_of_order_arrival_still_decrypts_every_message() {
    let (mut a, mut c) = (device("sec4-a"), device("sec4-c"));
    let (c_in_a, a_in_c) = pair(&mut a, "A", &mut c, "C");
    let texts = ["first", "second", "third"];
    let mut envs = Vec::new();
    for (i, t) in texts.iter().enumerate() {
        let before: Vec<_> = queue(&a)
            .into_iter()
            .map(|(_, e)| e.header.envelope_id)
            .collect();
        a.node.send_text(&c_in_a, t, T0 + i as u64).unwrap();
        envs.extend(
            queue(&a)
                .into_iter()
                .map(|(_, e)| e)
                .filter(|e| !before.contains(&e.header.envelope_id)),
        );
    }
    let mut wire = Wire::connect(&mut c, T0 + 60);
    for e in envs.iter().rev() {
        assert_eq!(wire.push(&mut c, e, T0 + 60), CustodyStatus::DeliveredLocal);
    }
    let mut got = bodies(&c, &a_in_c);
    got.sort();
    let mut want: Vec<_> = texts.iter().map(|s| (*s).to_owned()).collect();
    want.sort();
    assert_eq!(got, want);
}

#[test]
fn other_contacts_cannot_read_or_speak_as_the_sender() {
    let (mut a, mut c, mut d) = (device("sec5-a"), device("sec5-c"), device("sec5-d"));
    let (c_in_a, a_in_c) = pair(&mut a, "A", &mut c, "C");
    let (_d_in_a, a_in_d) = pair(&mut a, "A", &mut d, "D");
    let (_c_in_d, d_in_c) = pair(&mut d, "D", &mut c, "C");

    // D, a contact of A, receives A's message for C: it can only carry it.
    a.node.send_text(&c_in_a, CANARY, T0).unwrap();
    let for_c = own_envelope(&a);
    let mut wire = Wire::connect(&mut d, T0 + 60);
    let status = wire.push(&mut d, &for_c, T0 + 60);
    assert_ne!(status, CustodyStatus::DeliveredLocal, "not addressed to D");
    assert!(bodies(&d, &a_in_d).is_empty());
    assert!(d.node.search("CANARY", None, 10).unwrap().is_empty());

    // D writes "as A": it only has its own keys, so C files it under D.
    let d_c = d
        .node
        .contacts()
        .unwrap()
        .into_iter()
        .find(|x| x.display_name == "C")
        .unwrap()
        .contact_id;
    d.node.send_text(&d_c, "This is A, trust me", T0).unwrap();
    encounter(&mut d, &mut c, T0 + 120, MTU);
    assert!(bodies(&c, &d_in_c).contains(&"This is A, trust me".to_owned()));
    assert!(bodies(&c, &a_in_c).is_empty(), "nothing attributed to A");
}

#[test]
fn a_relay_cannot_claim_delivery() {
    let (mut a, mut c) = (device("sec6-a"), device("sec6-c"));
    let (c_in_a, _) = pair(&mut a, "A", &mut c, "C");
    let mid = a.node.send_text(&c_in_a, CANARY, T0).unwrap();
    let genuine = own_envelope(&a);

    // A malicious relay tells A "delivered" in every way it can without C's keys.
    let mut wire = Wire::connect(&mut a, T0 + 60);
    wire.send(
        &mut a,
        &LinkMessage::Custody(vec![CustodyItem {
            envelope_id: genuine.header.envelope_id,
            status: CustodyStatus::DeliveredLocal,
        }]),
        T0 + 60,
    );
    assert_ne!(state(&a, &c_in_a, &mid), MessageState::Acknowledged);

    // C really receives it and makes a receipt; a tampered receipt is refused.
    let mut to_c = Wire::connect(&mut c, T0 + 60);
    to_c.push(&mut c, &genuine, T0 + 60);
    let receipt = own_envelope(&c);
    let mut forged = receipt.clone();
    forged.body[20] ^= 0x80;
    wire.push(&mut a, &forged, T0 + 61);
    assert_ne!(state(&a, &c_in_a, &mid), MessageState::Acknowledged);
    // Only C's genuine receipt proves delivery.
    wire.push(&mut a, &receipt, T0 + 62);
    assert_eq!(state(&a, &c_in_a, &mid), MessageState::Acknowledged);
}

#[test]
fn expired_mail_is_refused_even_if_a_relay_held_it_back() {
    let (mut a, mut c) = (device("sec7-a"), device("sec7-c"));
    let (c_in_a, a_in_c) = pair(&mut a, "A", &mut c, "C");
    a.node.send_text(&c_in_a, CANARY, T0).unwrap();
    let genuine = own_envelope(&a);
    let late = u64::from(genuine.header.expiry_min) * 60 + 60;
    let mut wire = Wire::connect(&mut c, late);
    assert_eq!(
        wire.push(&mut c, &genuine, late),
        CustodyStatus::RejectedPolicy
    );
    assert!(bodies(&c, &a_in_c).is_empty());
}

#[test]
fn corrupted_attachment_parts_are_rejected_and_good_ones_complete_it() {
    let (mut a, mut c) = (device("sec8-a"), device("sec8-c"));
    let (c_in_a, _) = pair(&mut a, "A", &mut c, "C");
    let file: Vec<u8> = (0..5_000u32).map(|i| (i % 251) as u8).collect();
    let att = a
        .node
        .send_attachment(&c_in_a, ATTACHMENT_FILE, "notes.bin", &file, "", None, T0)
        .unwrap();
    let envs: Vec<_> = queue(&a).into_iter().map(|(_, e)| e).collect();
    let mut wire = Wire::connect(&mut c, T0 + 60);
    for e in &envs {
        let mut bad = e.clone();
        let last = bad.body.len() - 1;
        bad.body[last] ^= 0xFF;
        wire.push(&mut c, &bad, T0 + 60);
    }
    assert!(
        c.node.attachment_data(&att).unwrap().is_none(),
        "nothing assembled from bad parts"
    );
    for e in &envs {
        wire.push(&mut c, e, T0 + 61);
    }
    assert_eq!(c.node.attachment_data(&att).unwrap().unwrap(), file);
}

#[test]
fn a_lost_phone_cannot_be_silently_replaced() {
    let (mut a, mut c) = (device("sec9-a"), device("sec9-c"));
    let (c_in_a, _) = pair(&mut a, "A", &mut c, "C");
    let old = a
        .node
        .contacts()
        .unwrap()
        .into_iter()
        .find(|x| x.contact_id == c_in_a)
        .unwrap();

    // C loses the phone; a new phone (new keys) cannot read mail for the old one.
    let mut c2 = device("sec9-c2");
    a.node.send_text(&c_in_a, CANARY, T0).unwrap();
    let for_old_c = own_envelope(&a);
    let mut wire = Wire::connect(&mut c2, T0 + 60);
    assert_ne!(
        wire.push(&mut c2, &for_old_c, T0 + 60),
        CustodyStatus::DeliveredLocal
    );
    assert!(c2.node.search("CANARY", None, 10).unwrap().is_empty());

    // Pairing the new phone in person creates a separate contact with a new
    // safety number and ID, so a swap is visible; nothing is re-keyed silently.
    let (c2_in_a, _) = pair(&mut a, "A", &mut c2, "C");
    assert_ne!(c2_in_a, c_in_a);
    let new = a
        .node
        .contacts()
        .unwrap()
        .into_iter()
        .find(|x| x.contact_id == c2_in_a)
        .unwrap();
    assert_ne!(new.safety_number, old.safety_number);
    assert_ne!(new.id_code, old.id_code);
}
