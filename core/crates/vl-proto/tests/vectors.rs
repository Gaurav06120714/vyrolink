#![allow(clippy::unwrap_used, clippy::indexing_slicing)]
//! Golden wire vectors shared with the Swift and Kotlin hosts (`core/vectors/proto-v1.txt`).
//! Regenerate deliberately with `VL_REGEN_VECTORS=1 cargo test -p vl-proto --test vectors`.

use std::fmt::Write as _;
use vl_proto::*;

const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../vectors/proto-v1.txt");

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn header() -> Header {
    Header {
        kind: EnvelopeKind::Data,
        flags: FLAG_VACCINE,
        hop_count: 2,
        hop_limit: DEFAULT_HOP_LIMIT,
        copy_budget: 4,
        envelope_id: core::array::from_fn(|i| i as u8),
        recipient_tag: core::array::from_fn(|i| 0x10 + i as u8),
        expiry_min: 29_400_000,
        ack_commitment: core::array::from_fn(|i| 0x20 + i as u8),
        body_len: 256,
    }
}

fn valid() -> Vec<(&'static str, Vec<u8>)> {
    let envelope = Envelope {
        header: header(),
        body: vec![0xAB; 256],
    };
    vec![
        ("header", header().encode().to_vec()),
        ("header_aad", header().aad().to_vec()),
        ("envelope_256", envelope.encode()),
        (
            "link_hello",
            LinkMessage::Hello(Hello {
                link_versions: vec![LINK_VERSION],
                max_frame: MAX_FRAME_LEN as u16,
                relay_class: RelayClass::AlwaysOn,
                relay_willing: true,
                free_capacity_bucket: 128,
                l2cap_psm: None,
            })
            .encode()
            .unwrap(),
        ),
        (
            "link_summary",
            LinkMessage::Summary {
                entries: vec![SummaryEntry {
                    envelope_id: [1; 16],
                    recipient_tag: [2; 16],
                    expiry_min: 29_400_000,
                    body_len: 1024,
                    spray: true,
                }],
                delivered: vec![[3; 16]],
            }
            .encode()
            .unwrap(),
        ),
        (
            "link_custody",
            LinkMessage::Custody(vec![CustodyItem {
                envelope_id: [1; 16],
                status: CustodyStatus::Accepted,
            }])
            .encode()
            .unwrap(),
        ),
        (
            "link_vaccine",
            LinkMessage::Vaccine(vec![VaccineItem {
                ack_secret: [9; 16],
                expiry_min: 29_400_000,
            }])
            .encode()
            .unwrap(),
        ),
        (
            "link_envelope",
            LinkMessage::Envelope(envelope).encode().unwrap(),
        ),
        (
            "inner_text_padded_228",
            Inner {
                kind: InnerKind::Text,
                message_id: [0x11; 16],
                ack_secret: [0x22; 16],
                sent_at_ms: 1_764_000_000_000,
                payload: b"hi".to_vec(),
            }
            .encode_padded(228)
            .unwrap(),
        ),
        (
            "receipt",
            Receipt {
                refs: vec![[0x11; 16]],
                at_ms: 1_764_000_000_123,
            }
            .encode()
            .unwrap(),
        ),
        ("frame_hello", frame(b"hello").unwrap()),
        (
            "content_text_reply_timer",
            Content::Text {
                reply_to: Some([0x33; 16]),
                forwarded: false,
                timer_secs: 86_400,
                text: "ok".into(),
            }
            .encode()
            .unwrap(),
        ),
        (
            "content_reaction",
            Content::Reaction {
                target: [0x44; 16],
                reaction: 6,
            }
            .encode()
            .unwrap(),
        ),
        (
            "content_edit",
            Content::Edit {
                target: [0x55; 16],
                seq: 2,
                text: "fixed".into(),
            }
            .encode()
            .unwrap(),
        ),
        (
            "content_delete",
            Content::Delete { target: [0x66; 16] }.encode().unwrap(),
        ),
        (
            "content_timer",
            Content::Timer { timer_secs: 3600 }.encode().unwrap(),
        ),
        (
            "content_group_state",
            Content::GroupState {
                group_id: [0x71; 16],
                epoch: 2,
                name: "trip".into(),
                members: vec![
                    GroupMember {
                        fingerprint: [0x72; 32],
                        name: "A".into(),
                    },
                    GroupMember {
                        fingerprint: [0x73; 32],
                        name: "B".into(),
                    },
                ],
            }
            .encode()
            .unwrap(),
        ),
        (
            "content_group_text",
            Content::GroupText {
                group_id: [0x71; 16],
                group_msg_id: [0x74; 16],
                epoch: 2,
                reply_to: None,
                text: "hi all".into(),
            }
            .encode()
            .unwrap(),
        ),
        (
            "content_group_leave",
            Content::GroupLeave {
                group_id: [0x71; 16],
            }
            .encode()
            .unwrap(),
        ),
        (
            "content_attachment_image",
            Content::Attachment {
                attachment_id: [0x81; 16],
                kind: ATTACHMENT_IMAGE,
                total_len: 3000,
                sha256: [0x82; 32],
                key: [0x85; 32],
                reply_to: Some([0x83; 16]),
                name: "gate.jpg".into(),
                caption: "north gate".into(),
            }
            .encode()
            .unwrap(),
        ),
        (
            "attachment_chunk_payload",
            ChunkPayload {
                attachment_id: [0x81; 16],
                index: 1,
                ciphertext: vec![0x84; 24],
            }
            .encode()
            .unwrap(),
        ),
    ]
}

fn rejects() -> Vec<(&'static str, &'static str, Vec<u8>)> {
    let good = header().encode();
    let with = |i: usize, v: u8| {
        let mut b = good.to_vec();
        b[i] = v;
        b
    };
    vec![
        ("header_bad_version", "UnsupportedVersion", with(0, 2)),
        ("header_unknown_kind", "UnknownType", with(1, 7)),
        ("header_reserved_flag", "ReservedBitsSet", with(2, 0x81)),
        ("header_reserved_bytes", "ReservedBitsSet", with(6, 1)),
        ("header_bad_bucket", "BadLength", with(61, 0x01)),
        ("header_truncated", "Truncated", good[..61].to_vec()),
        ("link_unknown_type", "UnknownType", vec![0xEE]),
        (
            "link_request_count_too_large",
            "TooManyItems",
            vec![0x03, 0xFF, 0xFF],
        ),
        (
            "link_ping_trailing",
            "TrailingBytes",
            vec![0x07, 0, 0, 0, 1, 0],
        ),
        ("content_unknown_type", "UnknownType", vec![0x01, 0x7F]),
        (
            "chunk_payload_too_long",
            "InvalidField",
            [vec![0x81; 16], vec![0, 0, 0x05, 0x89]].concat(),
        ),
        (
            "content_reaction_out_of_range",
            "InvalidField",
            [vec![0x01, 0x02], vec![0x44; 16], vec![9]].concat(),
        ),
        (
            "content_timer_too_short",
            "InvalidField",
            vec![0x01, 0x05, 0, 0, 0, 59],
        ),
        (
            "content_text_reserved_flag",
            "ReservedBitsSet",
            vec![0x01, 0x01, 0x04, 0, 0, 0, 0, 0, 0],
        ),
        (
            "content_group_state_no_members",
            "TooManyItems",
            [vec![0x01, 0x06], vec![0x71; 16], vec![0, 0, 0, 1, 0, 0]].concat(),
        ),
    ]
}

fn error_name(e: ProtoError) -> &'static str {
    match e {
        ProtoError::Truncated => "Truncated",
        ProtoError::TrailingBytes => "TrailingBytes",
        ProtoError::UnsupportedVersion(_) => "UnsupportedVersion",
        ProtoError::UnknownType(_) => "UnknownType",
        ProtoError::ReservedBitsSet => "ReservedBitsSet",
        ProtoError::BadLength => "BadLength",
        ProtoError::TooManyItems => "TooManyItems",
        ProtoError::InvalidField(_) => "InvalidField",
    }
}

fn render() -> String {
    let mut out =
        String::from("# VYRO LINK protocol v1 test vectors. Format: <name> <expectation> <hex>\n");
    for (name, bytes) in valid() {
        let _ = writeln!(out, "{name} ok {}", hex(&bytes));
    }
    for (name, err, bytes) in rejects() {
        let _ = writeln!(out, "{name} reject:{err} {}", hex(&bytes));
    }
    out
}

#[test]
fn vectors_file_matches_encoder() {
    let rendered = render();
    if std::env::var_os("VL_REGEN_VECTORS").is_some() {
        std::fs::write(PATH, &rendered).unwrap();
    }
    let on_disk =
        std::fs::read_to_string(PATH).expect("vectors file missing; run with VL_REGEN_VECTORS=1");
    assert_eq!(
        on_disk, rendered,
        "wire format changed: review and regenerate vectors deliberately"
    );
}

#[test]
fn reject_vectors_fail_with_expected_error() {
    for line in std::fs::read_to_string(PATH)
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#'))
    {
        let mut parts = line.split(' ');
        let (name, expect, data) = (
            parts.next().unwrap(),
            parts.next().unwrap(),
            unhex(parts.next().unwrap()),
        );
        let Some(err) = expect.strip_prefix("reject:") else {
            continue;
        };
        let got = if name.starts_with("header") {
            Header::decode(&data).err()
        } else if name.starts_with("content") {
            Content::decode(&data).err()
        } else if name.starts_with("chunk") {
            ChunkPayload::decode(&data).err()
        } else {
            LinkMessage::decode(&data).err()
        };
        assert_eq!(got.map(error_name), Some(err), "{name}");
    }
}
