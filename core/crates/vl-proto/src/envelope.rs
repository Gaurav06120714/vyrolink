use crate::{ProtoError, Reader};

pub const PROTOCOL_VERSION: u8 = 1;
pub const HEADER_LEN: usize = 62;
/// Permitted body lengths. Bodies are padded to one of these so relays only learn a size bucket.
pub const BODY_BUCKETS: [u16; 4] = [256, 512, 1024, 2048];
pub const MAX_ENVELOPE_LEN: usize = HEADER_LEN + 2048;
/// Relays clamp copy budgets to this value (Doc 18 §7.2).
pub const MAX_COPY_BUDGET: u8 = 8;
pub const DEFAULT_HOP_LIMIT: u8 = 8;
/// ChaCha20-Poly1305 nonce (12) + tag (16) carried inside the body.
pub const AEAD_OVERHEAD: usize = 28;
pub const FLAG_VACCINE: u8 = 0b0000_0001;
const KNOWN_FLAGS: u8 = FLAG_VACCINE;

const OFF_HOP_COUNT: usize = 3;
const OFF_COPY_BUDGET: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnvelopeKind {
    Data = 1,
    Receipt = 2,
}

impl EnvelopeKind {
    fn from_u8(v: u8) -> Result<Self, ProtoError> {
        match v {
            1 => Ok(Self::Data),
            2 => Ok(Self::Receipt),
            other => Err(ProtoError::UnknownType(other)),
        }
    }
}

/// Smallest bucket that fits `body_len` bytes, if any.
pub fn bucket_for(body_len: usize) -> Option<u16> {
    BODY_BUCKETS
        .iter()
        .copied()
        .find(|b| usize::from(*b) >= body_len)
}

/// Relay-visible envelope header. `hop_count` and `copy_budget` are the only
/// fields a relay may change; everything else is authenticated as AEAD
/// associated data by the recipient (see [`Header::aad`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub kind: EnvelopeKind,
    pub flags: u8,
    pub hop_count: u8,
    pub hop_limit: u8,
    pub copy_budget: u8,
    pub envelope_id: [u8; 16],
    pub recipient_tag: [u8; 16],
    /// Unix time in minutes.
    pub expiry_min: u32,
    pub ack_commitment: [u8; 16],
    pub body_len: u16,
}

/// Result of a relay admission check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    Accept,
    Expired,
    LifetimeTooLong,
    HopLimitReached,
    NoBudget,
}

impl Header {
    pub fn encode(&self) -> [u8; HEADER_LEN] {
        let mut out = [0u8; HEADER_LEN];
        let mut w = Vec::with_capacity(HEADER_LEN);
        w.push(PROTOCOL_VERSION);
        w.push(self.kind as u8);
        w.push(self.flags);
        w.push(self.hop_count);
        w.push(self.hop_limit);
        w.push(self.copy_budget);
        w.extend_from_slice(&[0, 0]);
        w.extend_from_slice(&self.envelope_id);
        w.extend_from_slice(&self.recipient_tag);
        w.extend_from_slice(&self.expiry_min.to_be_bytes());
        w.extend_from_slice(&self.ack_commitment);
        w.extend_from_slice(&self.body_len.to_be_bytes());
        out.copy_from_slice(&w);
        out
    }

    /// Structural decode of exactly [`HEADER_LEN`] bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, ProtoError> {
        let mut r = Reader::new(bytes);
        let version = r.u8()?;
        if version != PROTOCOL_VERSION {
            return Err(ProtoError::UnsupportedVersion(version));
        }
        let kind = EnvelopeKind::from_u8(r.u8()?)?;
        let flags = r.u8()?;
        if flags & !KNOWN_FLAGS != 0 {
            return Err(ProtoError::ReservedBitsSet);
        }
        let hop_count = r.u8()?;
        let hop_limit = r.u8()?;
        let copy_budget = r.u8()?;
        if r.u16()? != 0 {
            return Err(ProtoError::ReservedBitsSet);
        }
        let header = Self {
            kind,
            flags,
            hop_count,
            hop_limit,
            copy_budget,
            envelope_id: r.array()?,
            recipient_tag: r.array()?,
            expiry_min: r.u32()?,
            ack_commitment: r.array()?,
            body_len: r.u16()?,
        };
        r.finish()?;
        if !BODY_BUCKETS.contains(&header.body_len) {
            return Err(ProtoError::BadLength);
        }
        Ok(header)
    }

    /// Associated data for the recipient's AEAD: the encoded header with the
    /// relay-mutable fields zeroed, so relays can update them without breaking
    /// authentication while every other header change is detected.
    pub fn aad(&self) -> [u8; HEADER_LEN] {
        let mut out = self.encode();
        if let Some(b) = out.get_mut(OFF_HOP_COUNT) {
            *b = 0;
        }
        if let Some(b) = out.get_mut(OFF_COPY_BUDGET) {
            *b = 0;
        }
        out
    }

    pub fn vaccine_enabled(&self) -> bool {
        self.flags & FLAG_VACCINE != 0
    }

    /// Relay admission policy (Doc 07 §3.2). `skew_min` tolerates unsynchronised clocks.
    pub fn admission(&self, now_min: u32, max_lifetime_min: u32, skew_min: u32) -> Admission {
        if self.expiry_min <= now_min {
            Admission::Expired
        } else if self.expiry_min
            > now_min
                .saturating_add(max_lifetime_min)
                .saturating_add(skew_min)
        {
            Admission::LifetimeTooLong
        } else if self.hop_count >= self.hop_limit {
            Admission::HopLimitReached
        } else if self.copy_budget == 0 {
            Admission::NoBudget
        } else {
            Admission::Accept
        }
    }
}

/// A complete envelope: header plus opaque body of exactly `header.body_len` bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    pub header: Header,
    pub body: Vec<u8>,
}

impl Envelope {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_LEN + self.body.len());
        out.extend_from_slice(&self.header.encode());
        out.extend_from_slice(&self.body);
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ProtoError> {
        if bytes.len() > MAX_ENVELOPE_LEN {
            return Err(ProtoError::BadLength);
        }
        let mut r = Reader::new(bytes);
        let header = Header::decode(r.take(HEADER_LEN)?)?;
        let body = r.rest();
        if body.len() != usize::from(header.body_len) {
            return Err(ProtoError::BadLength);
        }
        Ok(Self {
            header,
            body: body.to_vec(),
        })
    }

    /// Increments the hop count and clamps the copy budget, as every relay does on accept.
    pub fn on_relay_accept(&mut self) {
        self.header.hop_count = self.header.hop_count.saturating_add(1);
        self.header.copy_budget = self.header.copy_budget.min(MAX_COPY_BUDGET);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Header {
        Header {
            kind: EnvelopeKind::Data,
            flags: FLAG_VACCINE,
            hop_count: 0,
            hop_limit: DEFAULT_HOP_LIMIT,
            copy_budget: 4,
            envelope_id: [1; 16],
            recipient_tag: [2; 16],
            expiry_min: 1_000_000,
            ack_commitment: [3; 16],
            body_len: 256,
        }
    }

    #[test]
    fn header_round_trip_and_layout() {
        let h = sample();
        let bytes = h.encode();
        assert_eq!(bytes.len(), HEADER_LEN);
        assert_eq!(&bytes[..8], &[1, 1, 1, 0, 8, 4, 0, 0]);
        assert_eq!(Header::decode(&bytes), Ok(h));
    }

    #[test]
    fn aad_ignores_only_mutable_fields() {
        let a = sample();
        let mut b = a.clone();
        b.hop_count = 5;
        b.copy_budget = 1;
        assert_eq!(a.aad(), b.aad());
        let mut c = a.clone();
        c.expiry_min += 1;
        assert_ne!(a.aad(), c.aad());
    }

    #[test]
    fn rejects_bad_headers() {
        let good = sample().encode();
        let mut v = good;
        v[0] = 2;
        assert_eq!(Header::decode(&v), Err(ProtoError::UnsupportedVersion(2)));
        let mut t = good;
        t[1] = 9;
        assert_eq!(Header::decode(&t), Err(ProtoError::UnknownType(9)));
        let mut f = good;
        f[2] = 0b10;
        assert_eq!(Header::decode(&f), Err(ProtoError::ReservedBitsSet));
        let mut r = good;
        r[7] = 1;
        assert_eq!(Header::decode(&r), Err(ProtoError::ReservedBitsSet));
        let mut l = good;
        l[61] = 1;
        assert_eq!(Header::decode(&l), Err(ProtoError::BadLength));
        assert_eq!(Header::decode(&good[..61]), Err(ProtoError::Truncated));
    }

    #[test]
    fn envelope_length_must_match_bucket() {
        let env = Envelope {
            header: sample(),
            body: vec![0; 256],
        };
        let bytes = env.encode();
        assert_eq!(Envelope::decode(&bytes), Ok(env));
        assert_eq!(
            Envelope::decode(&bytes[..bytes.len() - 1]),
            Err(ProtoError::BadLength)
        );
        let mut long = bytes.clone();
        long.push(0);
        assert_eq!(Envelope::decode(&long), Err(ProtoError::BadLength));
    }

    #[test]
    fn admission_rules() {
        let h = sample();
        let max = 7 * 24 * 60;
        assert_eq!(h.admission(999_000, max, 120), Admission::Accept);
        assert_eq!(h.admission(1_000_000, max, 120), Admission::Expired);
        assert_eq!(
            h.admission(1_000_000 - max - 121, max, 120),
            Admission::LifetimeTooLong
        );
        let mut hop = h.clone();
        hop.hop_count = 8;
        assert_eq!(hop.admission(999_000, max, 120), Admission::HopLimitReached);
        let mut nob = h;
        nob.copy_budget = 0;
        assert_eq!(nob.admission(999_000, max, 120), Admission::NoBudget);
    }

    #[test]
    fn relay_accept_clamps_and_counts() {
        let mut header = sample();
        header.copy_budget = 200;
        let mut env = Envelope {
            header,
            body: vec![0; 256],
        };
        env.on_relay_accept();
        assert_eq!(env.header.copy_budget, MAX_COPY_BUDGET);
        assert_eq!(env.header.hop_count, 1);
    }

    #[test]
    fn bucket_selection() {
        assert_eq!(bucket_for(0), Some(256));
        assert_eq!(bucket_for(257), Some(512));
        assert_eq!(bucket_for(2048), Some(2048));
        assert_eq!(bucket_for(2049), None);
    }
}
