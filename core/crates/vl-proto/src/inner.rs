use crate::{ProtoError, Reader};

pub const INNER_VERSION: u8 = 1;
pub const INNER_HEADER_LEN: usize = 44;
pub const MAX_RECEIPT_REFS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InnerKind {
    /// Payload is an Olm message carrying UTF-8 text.
    Text = 0x10,
    /// Payload is an Olm message carrying a [`crate::Content`] event.
    Content = 0x11,
    /// Payload is a [`crate::ChunkPayload`]: one attachment chunk encrypted
    /// with the attachment's one-time key, which travels inside the Olm
    /// `Attachment` content event (ADR-026). Not Olm, so chunks may arrive in
    /// any order and number without exhausting Olm's 40 skipped message keys.
    AttachmentChunk = 0x12,
    /// Payload is a [`Receipt`]; authenticated by the pairwise wrap only, so it
    /// can be produced while the device is locked.
    ReceiptDelivered = 0x20,
    /// Payload is an Olm message carrying a [`Receipt`].
    ReceiptRead = 0x21,
    /// Payload is a [`Receipt`]; wrap-only.
    ReceiptUndecryptable = 0x22,
    KeyUpdate = 0x30,
    Revoke = 0x31,
    Profile = 0x40,
}

impl InnerKind {
    fn from_u8(v: u8) -> Result<Self, ProtoError> {
        Ok(match v {
            0x10 => Self::Text,
            0x11 => Self::Content,
            0x12 => Self::AttachmentChunk,
            0x20 => Self::ReceiptDelivered,
            0x21 => Self::ReceiptRead,
            0x22 => Self::ReceiptUndecryptable,
            0x30 => Self::KeyUpdate,
            0x31 => Self::Revoke,
            0x40 => Self::Profile,
            other => return Err(ProtoError::UnknownType(other)),
        })
    }
}

/// The plaintext sealed inside an envelope body by the pairwise wrap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inner {
    pub kind: InnerKind,
    pub message_id: [u8; 16],
    /// Released as a delivery vaccine; its hash is the header's `ack_commitment`.
    pub ack_secret: [u8; 16],
    pub sent_at_ms: u64,
    pub payload: Vec<u8>,
}

impl Inner {
    /// Encodes and zero-pads to exactly `padded_len` bytes.
    pub fn encode_padded(&self, padded_len: usize) -> Result<Vec<u8>, ProtoError> {
        let payload_len = u16::try_from(self.payload.len()).map_err(|_| ProtoError::BadLength)?;
        if INNER_HEADER_LEN + self.payload.len() > padded_len {
            return Err(ProtoError::BadLength);
        }
        let mut out = Vec::with_capacity(padded_len);
        out.push(INNER_VERSION);
        out.push(self.kind as u8);
        out.extend_from_slice(&self.message_id);
        out.extend_from_slice(&self.ack_secret);
        out.extend_from_slice(&self.sent_at_ms.to_be_bytes());
        out.extend_from_slice(&payload_len.to_be_bytes());
        out.extend_from_slice(&self.payload);
        out.resize(padded_len, 0);
        Ok(out)
    }

    /// Decodes and requires the padding to be all zeroes.
    pub fn decode(bytes: &[u8]) -> Result<Self, ProtoError> {
        let mut r = Reader::new(bytes);
        let version = r.u8()?;
        if version != INNER_VERSION {
            return Err(ProtoError::UnsupportedVersion(version));
        }
        let kind = InnerKind::from_u8(r.u8()?)?;
        let message_id = r.array()?;
        let ack_secret = r.array()?;
        let sent_at_ms = r.u64()?;
        let len = usize::from(r.u16()?);
        let payload = r.take(len)?.to_vec();
        if r.rest().iter().any(|b| *b != 0) {
            return Err(ProtoError::InvalidField("padding"));
        }
        Ok(Self {
            kind,
            message_id,
            ack_secret,
            sent_at_ms,
            payload,
        })
    }
}

/// Body of the receipt inner kinds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub refs: Vec<[u8; 16]>,
    pub at_ms: u64,
}

impl Receipt {
    pub fn encode(&self) -> Result<Vec<u8>, ProtoError> {
        if self.refs.is_empty() || self.refs.len() > MAX_RECEIPT_REFS {
            return Err(ProtoError::TooManyItems);
        }
        let mut out = Vec::with_capacity(1 + self.refs.len() * 16 + 8);
        out.push(self.refs.len() as u8);
        for id in &self.refs {
            out.extend_from_slice(id);
        }
        out.extend_from_slice(&self.at_ms.to_be_bytes());
        Ok(out)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ProtoError> {
        let mut r = Reader::new(bytes);
        let n = usize::from(r.u8()?);
        if n == 0 || n > MAX_RECEIPT_REFS {
            return Err(ProtoError::TooManyItems);
        }
        let mut refs = Vec::with_capacity(n);
        for _ in 0..n {
            refs.push(r.array()?);
        }
        let at_ms = r.u64()?;
        r.finish()?;
        Ok(Self { refs, at_ms })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inner_round_trip_with_padding() {
        let inner = Inner {
            kind: InnerKind::Text,
            message_id: [1; 16],
            ack_secret: [2; 16],
            sent_at_ms: 1_760_000_000_000,
            payload: b"olm-bytes".to_vec(),
        };
        let bytes = inner.encode_padded(228).unwrap();
        assert_eq!(bytes.len(), 228);
        assert_eq!(Inner::decode(&bytes).unwrap(), inner);
    }

    #[test]
    fn rejects_nonzero_padding_and_overflow() {
        let inner = Inner {
            kind: InnerKind::ReceiptDelivered,
            message_id: [0; 16],
            ack_secret: [0; 16],
            sent_at_ms: 0,
            payload: vec![1; 10],
        };
        let mut bytes = inner.encode_padded(100).unwrap();
        *bytes.last_mut().unwrap() = 1;
        assert_eq!(
            Inner::decode(&bytes),
            Err(ProtoError::InvalidField("padding"))
        );
        assert_eq!(
            inner.encode_padded(INNER_HEADER_LEN + 9),
            Err(ProtoError::BadLength)
        );
        let mut long = inner.encode_padded(100).unwrap();
        long[42] = 0xFF;
        assert_eq!(Inner::decode(&long), Err(ProtoError::Truncated));
    }

    #[test]
    fn receipt_bounds() {
        let r = Receipt {
            refs: vec![[3; 16]; 2],
            at_ms: 5,
        };
        assert_eq!(Receipt::decode(&r.encode().unwrap()).unwrap(), r);
        assert_eq!(
            Receipt {
                refs: vec![],
                at_ms: 0
            }
            .encode(),
            Err(ProtoError::TooManyItems)
        );
        assert_eq!(Receipt::decode(&[17]), Err(ProtoError::TooManyItems));
    }
}
