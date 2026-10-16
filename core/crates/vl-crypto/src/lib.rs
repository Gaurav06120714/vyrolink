//! VYRO LINK cryptography. Composes audited primitives only; no new constructions:
//! X25519 + HKDF-SHA256 for pairwise keys, HMAC-SHA256 for tags,
//! ChaCha20-Poly1305 (RFC 8439) for the envelope wrap, Olm (vodozemac) for
//! end-to-end sessions, and `Noise_NN_25519_ChaChaPoly_SHA256` (snow) for links.
//! Security design: `research/06-security-privacy.md`, `research/18-development-blueprint.md` §8.

mod identity;
mod link;
mod message;
mod pairwise;
mod session;

pub use identity::{Identity, card_hash, verify_card};
pub use link::{LinkHandshake, LinkTransport, NOISE_PATTERN};
pub use message::{
    Opened, Outgoing, SendParams, build_attachment_chunk, build_content, build_content_bytes,
    build_content_bytes_with_id, build_receipt, build_text, olm_payload, open_chunk, open_from,
    seal_chunk, verify_receipt,
};
pub use pairwise::{
    EPOCH_SECONDS, PairKeys, ack_commitment, epoch_of, id_code, open, random_array, safety_number,
    seal, sha256,
};
pub use session::{OlmPayload, decrypt_olm, encrypt_olm};

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoError {
    Rng,
    BadSignature,
    InvalidKey,
    NonContributory,
    Aead,
    Olm(String),
    Noise(String),
    Proto(vl_proto::ProtoError),
    Pickle,
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rng => f.write_str("system randomness unavailable"),
            Self::BadSignature => f.write_str("signature verification failed"),
            Self::InvalidKey => f.write_str("invalid key"),
            Self::NonContributory => f.write_str("non-contributory key agreement"),
            Self::Aead => f.write_str("authenticated decryption failed"),
            Self::Olm(e) => write!(f, "olm: {e}"),
            Self::Noise(e) => write!(f, "noise: {e}"),
            Self::Proto(e) => write!(f, "encoding: {e}"),
            Self::Pickle => f.write_str("could not restore encrypted state"),
        }
    }
}

impl std::error::Error for CryptoError {}

impl From<vl_proto::ProtoError> for CryptoError {
    fn from(e: vl_proto::ProtoError) -> Self {
        Self::Proto(e)
    }
}
