//! VYRO LINK wire formats (protocol v1).
//!
//! Every decoder in this crate treats its input as hostile: lengths are checked
//! before use, item counts are capped, and no decoder allocates more than a
//! small constant multiple of its input length. Normative description:
//! `research/21-wire-protocol-v1.md`.

mod card;
mod content;
mod envelope;
mod frame;
mod inner;
mod link;
mod pairing_code;

pub use card::{MAX_NAME_LEN, MAX_OTKS, PairingCard};
pub use content::{
    ATTACHMENT_FILE, ATTACHMENT_IMAGE, CHUNK_BYTES, CHUNK_TAG_BYTES, CONTENT_VERSION, ChunkPayload,
    Content, GroupMember, GroupRef, MAX_ATTACHMENT_NAME, MAX_CAPTION_BYTES, MAX_EDIT_SEQ,
    MAX_FILE_BYTES, MAX_GROUP_MEMBERS, MAX_IMAGE_BYTES, MAX_POLL_OPTION_BYTES, MAX_POLL_OPTIONS,
    MAX_POLL_QUESTION_BYTES, MAX_TIMER_SECS, MIN_TIMER_SECS, REACTIONS, chunk_count_for,
};
pub use envelope::{
    AEAD_OVERHEAD, Admission, BODY_BUCKETS, DEFAULT_HOP_LIMIT, Envelope, EnvelopeKind,
    FLAG_VACCINE, HEADER_LEN, Header, MAX_COPY_BUDGET, MAX_ENVELOPE_LEN, PROTOCOL_VERSION,
    bucket_for,
};
pub use frame::{Deframer, frame};
pub use inner::{INNER_HEADER_LEN, Inner, InnerKind, MAX_RECEIPT_REFS, Receipt};
pub use link::{
    CustodyItem, CustodyStatus, FEATURE_PRESENCE, Hello, LINK_VERSION, LinkMessage, MAX_FRAME_LEN,
    MAX_ITEMS, PRESENCE_PAD, RelayClass, RequestItem, RequestRole, SummaryEntry, VaccineItem,
};
pub use pairing_code::{
    MAX_PAIRING_CODE_LEN, PAIRING_CODE_PREFIX, PAPER_PREFIX, decode_pairing_code,
    decode_paper_message, encode_pairing_code, encode_paper_message,
};

use std::fmt;

/// Why a byte string was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtoError {
    Truncated,
    TrailingBytes,
    UnsupportedVersion(u8),
    UnknownType(u8),
    ReservedBitsSet,
    BadLength,
    TooManyItems,
    InvalidField(&'static str),
}

impl fmt::Display for ProtoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => f.write_str("input truncated"),
            Self::TrailingBytes => f.write_str("unexpected trailing bytes"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported version {v}"),
            Self::UnknownType(t) => write!(f, "unknown type 0x{t:02x}"),
            Self::ReservedBitsSet => f.write_str("reserved bits set"),
            Self::BadLength => f.write_str("length outside permitted values"),
            Self::TooManyItems => f.write_str("item count exceeds limit"),
            Self::InvalidField(name) => write!(f, "invalid field: {name}"),
        }
    }
}

impl std::error::Error for ProtoError {}

/// Bounds-checked big-endian reader. Never panics.
#[derive(Debug)]
pub(crate) struct Reader<'a> {
    buf: &'a [u8],
}

impl<'a> Reader<'a> {
    pub(crate) fn new(buf: &'a [u8]) -> Self {
        Self { buf }
    }

    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8], ProtoError> {
        if self.buf.len() < n {
            return Err(ProtoError::Truncated);
        }
        let (head, tail) = self.buf.split_at(n);
        self.buf = tail;
        Ok(head)
    }

    pub(crate) fn array<const N: usize>(&mut self) -> Result<[u8; N], ProtoError> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, ProtoError> {
        Ok(self.array::<1>()?[0])
    }

    pub(crate) fn u16(&mut self) -> Result<u16, ProtoError> {
        Ok(u16::from_be_bytes(self.array()?))
    }

    pub(crate) fn u32(&mut self) -> Result<u32, ProtoError> {
        Ok(u32::from_be_bytes(self.array()?))
    }

    pub(crate) fn u64(&mut self) -> Result<u64, ProtoError> {
        Ok(u64::from_be_bytes(self.array()?))
    }

    pub(crate) fn rest(&mut self) -> &'a [u8] {
        std::mem::take(&mut self.buf)
    }

    pub(crate) fn remaining(&self) -> usize {
        self.buf.len()
    }

    pub(crate) fn finish(self) -> Result<(), ProtoError> {
        if self.buf.is_empty() {
            Ok(())
        } else {
            Err(ProtoError::TrailingBytes)
        }
    }

    /// Reads a u16 item count and rejects it before any allocation if it exceeds `max`.
    pub(crate) fn count(&mut self, max: usize) -> Result<usize, ProtoError> {
        let n = usize::from(self.u16()?);
        if n > max {
            return Err(ProtoError::TooManyItems);
        }
        Ok(n)
    }
}
