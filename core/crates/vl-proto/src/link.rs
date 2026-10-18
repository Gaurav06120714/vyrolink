use crate::envelope::{BODY_BUCKETS, Envelope, MAX_ENVELOPE_LEN};
use crate::{ProtoError, Reader};

pub const LINK_VERSION: u8 = 1;
/// Upper bound for every list carried in a link message.
pub const MAX_ITEMS: usize = 128;
const SUMMARY_ENTRY_LEN: usize = 16 + 16 + 4 + 1 + 1;
const MAX_SUMMARY_LEN: usize = 1 + 2 + MAX_ITEMS * SUMMARY_ENTRY_LEN + 2 + MAX_ITEMS * 16;
const MAX_ENVELOPE_MSG_LEN: usize = 1 + MAX_ENVELOPE_LEN;
/// Largest link-message plaintext any peer may send (a full SUMMARY: 6,917 bytes).
pub const MAX_FRAME_LEN: usize = if MAX_SUMMARY_LEN > MAX_ENVELOPE_MSG_LEN {
    MAX_SUMMARY_LEN
} else {
    MAX_ENVELOPE_MSG_LEN
};
const MAX_LINK_VERSIONS: usize = 8;
const MAX_TLV_BYTES: usize = 256;
/// HELLO extension carrying optional-feature bits (ADR-029). Older peers skip it.
const TLV_FEATURES: u8 = 0x01;
/// Feature bit: the peer understands PRESENCE (ADR-029).
pub const FEATURE_PRESENCE: u8 = 0b0000_0001;
/// Presence tags are padded with random values to a multiple of this, so a
/// peer learns only a coarse contact count.
pub const PRESENCE_PAD: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayClass {
    AlwaysOn = 0,
    BestEffort = 1,
    EndpointOnly = 2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hello {
    pub link_versions: Vec<u8>,
    pub max_frame: u16,
    pub relay_class: RelayClass,
    pub relay_willing: bool,
    /// Coarse free carry capacity, 0 (none) ..= 255 (plenty). Never an exact byte count.
    pub free_capacity_bucket: u8,
    pub l2cap_psm: Option<u16>,
    /// Optional-feature bits ([`FEATURE_PRESENCE`]); sent as a HELLO extension.
    pub features: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SummaryEntry {
    pub envelope_id: [u8; 16],
    pub recipient_tag: [u8; 16],
    pub expiry_min: u32,
    pub body_len: u16,
    /// True while the sender still holds a copy budget above one (spray phase).
    pub spray: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestRole {
    Deliver = 0,
    Carry = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestItem {
    pub envelope_id: [u8; 16],
    pub role: RequestRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustodyStatus {
    Accepted = 0,
    RejectedFull = 1,
    RejectedPolicy = 2,
    Duplicate = 3,
    /// Diagnostic only: an unauthenticated claim. Never sets "Delivered" (ADR-015).
    DeliveredLocal = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CustodyItem {
    pub envelope_id: [u8; 16],
    pub status: CustodyStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VaccineItem {
    pub ack_secret: [u8; 16],
    pub expiry_min: u32,
}

/// Plaintext of one Noise transport message on a link session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkMessage {
    Hello(Hello),
    Summary {
        entries: Vec<SummaryEntry>,
        delivered: Vec<[u8; 16]>,
    },
    Request(Vec<RequestItem>),
    Envelope(Envelope),
    Custody(Vec<CustodyItem>),
    Vaccine(Vec<VaccineItem>),
    Ping(u32),
    Pong(u32),
    Bye(u8),
    /// Contact-presence tags bound to this link (ADR-029). Sent only to peers
    /// whose HELLO set [`FEATURE_PRESENCE`].
    Presence(Vec<[u8; 16]>),
}

const T_HELLO: u8 = 0x01;
const T_SUMMARY: u8 = 0x02;
const T_REQUEST: u8 = 0x03;
const T_ENVELOPE: u8 = 0x04;
const T_CUSTODY: u8 = 0x05;
const T_VACCINE: u8 = 0x06;
const T_PING: u8 = 0x07;
const T_PONG: u8 = 0x08;
const T_BYE: u8 = 0x09;
const T_PRESENCE: u8 = 0x0A;

const HELLO_FLAG_RELAY: u8 = 0b01;
const HELLO_FLAG_L2CAP: u8 = 0b10;

fn bucket_index(body_len: u16) -> Result<u8, ProtoError> {
    BODY_BUCKETS
        .iter()
        .position(|b| *b == body_len)
        .and_then(|i| u8::try_from(i).ok())
        .ok_or(ProtoError::BadLength)
}

fn checked_count(n: usize) -> Result<u16, ProtoError> {
    if n > MAX_ITEMS {
        return Err(ProtoError::TooManyItems);
    }
    u16::try_from(n).map_err(|_| ProtoError::TooManyItems)
}

impl LinkMessage {
    pub fn encode(&self) -> Result<Vec<u8>, ProtoError> {
        let mut w = Vec::new();
        match self {
            Self::Hello(h) => {
                if h.link_versions.is_empty() || h.link_versions.len() > MAX_LINK_VERSIONS {
                    return Err(ProtoError::InvalidField("link_versions"));
                }
                w.push(T_HELLO);
                w.push(h.link_versions.len() as u8);
                w.extend_from_slice(&h.link_versions);
                w.extend_from_slice(&h.max_frame.to_be_bytes());
                w.push(h.relay_class as u8);
                let mut flags = 0;
                if h.relay_willing {
                    flags |= HELLO_FLAG_RELAY;
                }
                if h.l2cap_psm.is_some() {
                    flags |= HELLO_FLAG_L2CAP;
                }
                w.push(flags);
                w.push(h.free_capacity_bucket);
                w.extend_from_slice(&h.l2cap_psm.unwrap_or(0).to_be_bytes());
                if h.features != 0 {
                    w.extend_from_slice(&[TLV_FEATURES, 1, h.features]);
                }
            }
            Self::Summary { entries, delivered } => {
                w.push(T_SUMMARY);
                w.extend_from_slice(&checked_count(entries.len())?.to_be_bytes());
                for e in entries {
                    w.extend_from_slice(&e.envelope_id);
                    w.extend_from_slice(&e.recipient_tag);
                    w.extend_from_slice(&e.expiry_min.to_be_bytes());
                    w.push(bucket_index(e.body_len)?);
                    w.push(u8::from(e.spray));
                }
                w.extend_from_slice(&checked_count(delivered.len())?.to_be_bytes());
                for c in delivered {
                    w.extend_from_slice(c);
                }
            }
            Self::Request(items) => {
                w.push(T_REQUEST);
                w.extend_from_slice(&checked_count(items.len())?.to_be_bytes());
                for i in items {
                    w.extend_from_slice(&i.envelope_id);
                    w.push(i.role as u8);
                }
            }
            Self::Envelope(env) => {
                w.push(T_ENVELOPE);
                w.extend_from_slice(&env.encode());
            }
            Self::Custody(items) => {
                w.push(T_CUSTODY);
                w.extend_from_slice(&checked_count(items.len())?.to_be_bytes());
                for i in items {
                    w.extend_from_slice(&i.envelope_id);
                    w.push(i.status as u8);
                }
            }
            Self::Vaccine(items) => {
                w.push(T_VACCINE);
                w.extend_from_slice(&checked_count(items.len())?.to_be_bytes());
                for i in items {
                    w.extend_from_slice(&i.ack_secret);
                    w.extend_from_slice(&i.expiry_min.to_be_bytes());
                }
            }
            Self::Ping(t) => {
                w.push(T_PING);
                w.extend_from_slice(&t.to_be_bytes());
            }
            Self::Pong(t) => {
                w.push(T_PONG);
                w.extend_from_slice(&t.to_be_bytes());
            }
            Self::Bye(reason) => {
                w.push(T_BYE);
                w.push(*reason);
            }
            Self::Presence(tags) => {
                w.push(T_PRESENCE);
                w.extend_from_slice(&checked_count(tags.len())?.to_be_bytes());
                for t in tags {
                    w.extend_from_slice(t);
                }
            }
        }
        Ok(w)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ProtoError> {
        if bytes.len() > MAX_FRAME_LEN {
            return Err(ProtoError::BadLength);
        }
        let mut r = Reader::new(bytes);
        let msg = match r.u8()? {
            T_HELLO => {
                let n = usize::from(r.u8()?);
                if n == 0 || n > MAX_LINK_VERSIONS {
                    return Err(ProtoError::InvalidField("link_versions"));
                }
                let link_versions = r.take(n)?.to_vec();
                let max_frame = r.u16()?;
                let relay_class = match r.u8()? {
                    0 => RelayClass::AlwaysOn,
                    1 => RelayClass::BestEffort,
                    2 => RelayClass::EndpointOnly,
                    _ => return Err(ProtoError::InvalidField("relay_class")),
                };
                let flags = r.u8()?;
                if flags & !(HELLO_FLAG_RELAY | HELLO_FLAG_L2CAP) != 0 {
                    return Err(ProtoError::ReservedBitsSet);
                }
                let free_capacity_bucket = r.u8()?;
                let psm = r.u16()?;
                let features = read_tlvs(&mut r)?;
                return Ok(Self::Hello(Hello {
                    link_versions,
                    max_frame,
                    relay_class,
                    relay_willing: flags & HELLO_FLAG_RELAY != 0,
                    free_capacity_bucket,
                    l2cap_psm: (flags & HELLO_FLAG_L2CAP != 0).then_some(psm),
                    features,
                }));
            }
            T_SUMMARY => {
                let n = r.count(MAX_ITEMS)?;
                let mut entries = Vec::with_capacity(n);
                for _ in 0..n {
                    let envelope_id = r.array()?;
                    let recipient_tag = r.array()?;
                    let expiry_min = r.u32()?;
                    let body_len = *BODY_BUCKETS
                        .get(usize::from(r.u8()?))
                        .ok_or(ProtoError::BadLength)?;
                    let spray = match r.u8()? {
                        0 => false,
                        1 => true,
                        _ => return Err(ProtoError::InvalidField("spray")),
                    };
                    entries.push(SummaryEntry {
                        envelope_id,
                        recipient_tag,
                        expiry_min,
                        body_len,
                        spray,
                    });
                }
                let m = r.count(MAX_ITEMS)?;
                let mut delivered = Vec::with_capacity(m);
                for _ in 0..m {
                    delivered.push(r.array()?);
                }
                Self::Summary { entries, delivered }
            }
            T_REQUEST => {
                let n = r.count(MAX_ITEMS)?;
                let mut items = Vec::with_capacity(n);
                for _ in 0..n {
                    let envelope_id = r.array()?;
                    let role = match r.u8()? {
                        0 => RequestRole::Deliver,
                        1 => RequestRole::Carry,
                        _ => return Err(ProtoError::InvalidField("role")),
                    };
                    items.push(RequestItem { envelope_id, role });
                }
                Self::Request(items)
            }
            T_ENVELOPE => Self::Envelope(Envelope::decode(r.rest())?),
            T_CUSTODY => {
                let n = r.count(MAX_ITEMS)?;
                let mut items = Vec::with_capacity(n);
                for _ in 0..n {
                    let envelope_id = r.array()?;
                    let status = match r.u8()? {
                        0 => CustodyStatus::Accepted,
                        1 => CustodyStatus::RejectedFull,
                        2 => CustodyStatus::RejectedPolicy,
                        3 => CustodyStatus::Duplicate,
                        4 => CustodyStatus::DeliveredLocal,
                        _ => return Err(ProtoError::InvalidField("custody_status")),
                    };
                    items.push(CustodyItem {
                        envelope_id,
                        status,
                    });
                }
                Self::Custody(items)
            }
            T_VACCINE => {
                let n = r.count(MAX_ITEMS)?;
                let mut items = Vec::with_capacity(n);
                for _ in 0..n {
                    items.push(VaccineItem {
                        ack_secret: r.array()?,
                        expiry_min: r.u32()?,
                    });
                }
                Self::Vaccine(items)
            }
            T_PING => Self::Ping(r.u32()?),
            T_PONG => Self::Pong(r.u32()?),
            T_BYE => Self::Bye(r.u8()?),
            T_PRESENCE => {
                let n = r.count(MAX_ITEMS)?;
                let mut tags = Vec::with_capacity(n);
                for _ in 0..n {
                    tags.push(r.array()?);
                }
                Self::Presence(tags)
            }
            other => return Err(ProtoError::UnknownType(other)),
        };
        r.finish()?;
        Ok(msg)
    }
}

/// HELLO may end with (type u8, len u8, value) extension fields. All are
/// validated for shape; the features field is read and unknown ones are
/// ignored, so newer peers can add fields without breaking older ones.
fn read_tlvs(r: &mut Reader<'_>) -> Result<u8, ProtoError> {
    if r.remaining() > MAX_TLV_BYTES {
        return Err(ProtoError::BadLength);
    }
    let mut features = 0;
    while r.remaining() > 0 {
        let kind = r.u8()?;
        let len = usize::from(r.u8()?);
        let value = r.take(len)?;
        if kind == TLV_FEATURES {
            // Longer values are allowed for future bits; only the first byte is known.
            features = value.first().copied().unwrap_or(0);
        }
    }
    Ok(features)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::{EnvelopeKind, Header};

    fn hello() -> Hello {
        Hello {
            link_versions: vec![1],
            max_frame: 2111,
            relay_class: RelayClass::BestEffort,
            relay_willing: true,
            free_capacity_bucket: 200,
            l2cap_psm: Some(0x0081),
            features: 0,
        }
    }

    #[test]
    fn hello_round_trip_and_ignores_future_tlvs() {
        let msg = LinkMessage::Hello(hello());
        let mut bytes = msg.encode().unwrap();
        assert_eq!(LinkMessage::decode(&bytes).unwrap(), msg);
        bytes.extend_from_slice(&[0x10, 2, 0xAA, 0xBB]);
        assert_eq!(LinkMessage::decode(&bytes).unwrap(), msg);
        bytes.extend_from_slice(&[0x11, 5, 1]);
        assert_eq!(LinkMessage::decode(&bytes), Err(ProtoError::Truncated));
    }

    #[test]
    fn hello_features_extension_and_presence_bounds() {
        let msg = LinkMessage::Hello(Hello {
            features: FEATURE_PRESENCE,
            ..hello()
        });
        let bytes = msg.encode().unwrap();
        assert_eq!(
            &bytes[bytes.len() - 3..],
            &[TLV_FEATURES, 1, FEATURE_PRESENCE]
        );
        assert_eq!(LinkMessage::decode(&bytes).unwrap(), msg);
        // Without features nothing is appended, so older HELLO bytes are unchanged.
        let plain = LinkMessage::Hello(hello()).encode().unwrap();
        assert_eq!(plain.len() + 3, bytes.len());
        // A longer features value (future bits) still reads the first byte.
        let mut longer = plain.clone();
        longer.extend_from_slice(&[TLV_FEATURES, 2, FEATURE_PRESENCE, 0xFF]);
        assert_eq!(LinkMessage::decode(&longer).unwrap(), msg);

        let too_many = LinkMessage::Presence(vec![[0; 16]; MAX_ITEMS + 1]);
        assert_eq!(too_many.encode(), Err(ProtoError::TooManyItems));
        let mut raw = vec![T_PRESENCE];
        raw.extend_from_slice(&u16::try_from(MAX_ITEMS + 1).unwrap().to_be_bytes());
        assert!(LinkMessage::decode(&raw).is_err());
        let truncated = [T_PRESENCE, 0, 1, 9, 9];
        assert_eq!(LinkMessage::decode(&truncated), Err(ProtoError::Truncated));
    }

    #[test]
    fn every_message_round_trips() {
        let env = Envelope {
            header: Header {
                kind: EnvelopeKind::Receipt,
                flags: 0,
                hop_count: 1,
                hop_limit: 8,
                copy_budget: 2,
                envelope_id: [9; 16],
                recipient_tag: [8; 16],
                expiry_min: 77,
                ack_commitment: [0; 16],
                body_len: 512,
            },
            body: vec![5; 512],
        };
        let msgs = vec![
            LinkMessage::Hello(hello()),
            LinkMessage::Summary {
                entries: vec![SummaryEntry {
                    envelope_id: [1; 16],
                    recipient_tag: [2; 16],
                    expiry_min: 3,
                    body_len: 1024,
                    spray: true,
                }],
                delivered: vec![[4; 16]],
            },
            LinkMessage::Request(vec![RequestItem {
                envelope_id: [1; 16],
                role: RequestRole::Carry,
            }]),
            LinkMessage::Envelope(env),
            LinkMessage::Custody(vec![CustodyItem {
                envelope_id: [1; 16],
                status: CustodyStatus::Duplicate,
            }]),
            LinkMessage::Vaccine(vec![VaccineItem {
                ack_secret: [6; 16],
                expiry_min: 99,
            }]),
            LinkMessage::Ping(1),
            LinkMessage::Pong(2),
            LinkMessage::Bye(3),
            LinkMessage::Presence(vec![[7; 16], [8; 16]]),
            LinkMessage::Presence(vec![]),
        ];
        for m in msgs {
            let bytes = m.encode().unwrap();
            assert!(bytes.len() <= MAX_FRAME_LEN);
            assert_eq!(LinkMessage::decode(&bytes).unwrap(), m);
        }
    }

    #[test]
    fn item_counts_are_capped_before_allocation() {
        let mut bytes = vec![T_REQUEST];
        bytes.extend_from_slice(&u16::MAX.to_be_bytes());
        assert_eq!(LinkMessage::decode(&bytes), Err(ProtoError::TooManyItems));
        let too_many = LinkMessage::Request(vec![
            RequestItem {
                envelope_id: [0; 16],
                role: RequestRole::Deliver
            };
            MAX_ITEMS + 1
        ]);
        assert_eq!(too_many.encode(), Err(ProtoError::TooManyItems));
    }

    #[test]
    fn rejects_unknown_and_trailing() {
        assert_eq!(
            LinkMessage::decode(&[0xEE]),
            Err(ProtoError::UnknownType(0xEE))
        );
        assert_eq!(
            LinkMessage::decode(&[T_PING, 0, 0, 0, 1, 9]),
            Err(ProtoError::TrailingBytes)
        );
        assert_eq!(LinkMessage::decode(&[]), Err(ProtoError::Truncated));
    }
}

#[cfg(test)]
mod limit_tests {
    use super::*;

    #[test]
    fn frame_limit_covers_largest_message() {
        assert_eq!(MAX_FRAME_LEN, 6917);
        let full = LinkMessage::Summary {
            entries: vec![
                SummaryEntry {
                    envelope_id: [0; 16],
                    recipient_tag: [0; 16],
                    expiry_min: 0,
                    body_len: 2048,
                    spray: true
                };
                MAX_ITEMS
            ],
            delivered: vec![[0; 16]; MAX_ITEMS],
        };
        assert_eq!(full.encode().unwrap().len(), MAX_FRAME_LEN);
    }
}
