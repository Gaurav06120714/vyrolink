use crate::CryptoError;
use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, KeyInit, Nonce};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256, Sha512};
use std::fmt;
use vl_proto::{AEAD_OVERHEAD, Header};
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Recipient-tag rotation period (ADR-009).
pub const EPOCH_SECONDS: u64 = 6 * 60 * 60;
const NONCE_LEN: usize = 12;

pub fn epoch_of(unix_secs: u64) -> u32 {
    u32::try_from(unix_secs / EPOCH_SECONDS).unwrap_or(u32::MAX)
}

pub fn random_array<const N: usize>() -> Result<[u8; N], CryptoError> {
    let mut out = [0u8; N];
    getrandom::fill(&mut out).map_err(|_| CryptoError::Rng)?;
    Ok(out)
}

/// Directional pairwise keys established at in-person pairing (Doc 06 §3.2).
/// `*_out` keys protect what this device sends to the contact; `*_in` keys what it receives.
#[derive(Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct PairKeys {
    tag_out: [u8; 32],
    tag_in: [u8; 32],
    wrap_out: [u8; 32],
    wrap_in: [u8; 32],
}

impl fmt::Debug for PairKeys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PairKeys(<redacted>)")
    }
}

impl PairKeys {
    /// `K_pair = HKDF-Extract(SHA-256(min(Fa,Fb) ‖ max(Fa,Fb)), X25519(ak_self, ak_peer))`;
    /// each directional key is `HKDF-Expand(K_pair, label ‖ F_sender ‖ F_recipient)`.
    /// `F` is each side's Ed25519 identity (fingerprint) key, taken from the
    /// signed cards (ADR-031); it is the same for every card a phone shows.
    pub fn derive(
        my_addressing: &StaticSecret,
        my_id: &[u8; 32],
        their_addressing_pub: &[u8; 32],
        their_id: &[u8; 32],
    ) -> Result<Self, CryptoError> {
        if my_id == their_id {
            return Err(CryptoError::InvalidKey);
        }
        let shared = my_addressing.diffie_hellman(&PublicKey::from(*their_addressing_pub));
        if !shared.was_contributory() {
            return Err(CryptoError::NonContributory);
        }
        let (lo, hi) = if my_id < their_id {
            (my_id, their_id)
        } else {
            (their_id, my_id)
        };
        let salt = Sha256::new().chain_update(lo).chain_update(hi).finalize();
        let hk = Hkdf::<Sha256>::new(Some(salt.as_slice()), shared.as_bytes());
        let expand =
            |label: &[u8], from: &[u8; 32], to: &[u8; 32]| -> Result<[u8; 32], CryptoError> {
                let mut info = Vec::with_capacity(label.len() + 64);
                info.extend_from_slice(label);
                info.extend_from_slice(from);
                info.extend_from_slice(to);
                let mut okm = [0u8; 32];
                hk.expand(&info, &mut okm)
                    .map_err(|_| CryptoError::InvalidKey)?;
                Ok(okm)
            };
        Ok(Self {
            tag_out: expand(b"VL/v1/tag", my_id, their_id)?,
            tag_in: expand(b"VL/v1/tag", their_id, my_id)?,
            wrap_out: expand(b"VL/v1/wrap", my_id, their_id)?,
            wrap_in: expand(b"VL/v1/wrap", their_id, my_id)?,
        })
    }

    pub fn tag_out(&self, epoch: u32) -> [u8; 16] {
        tag(&self.tag_out, epoch)
    }

    pub fn tag_in(&self, epoch: u32) -> [u8; 16] {
        tag(&self.tag_in, epoch)
    }

    /// Presence tag this device shows the contact on one link (ADR-029):
    /// `Trunc16(HMAC(tag_out, "VL/presence/v1" ‖ h))`, where `h` is the link's
    /// Noise handshake hash. Fresh per link, so strangers can't link sightings.
    pub fn presence_out(&self, handshake_hash: &[u8; 32]) -> [u8; 16] {
        presence(&self.tag_out, handshake_hash)
    }

    /// The tag the contact would show this device on the same link.
    pub fn presence_in(&self, handshake_hash: &[u8; 32]) -> [u8; 16] {
        presence(&self.tag_in, handshake_hash)
    }

    pub fn wrap_out(&self) -> &[u8; 32] {
        &self.wrap_out
    }

    pub fn wrap_in(&self) -> &[u8; 32] {
        &self.wrap_in
    }

    /// Serialised form for the Class R store (which is itself encrypted at rest).
    pub fn to_bytes(&self) -> [u8; 128] {
        let mut out = [0u8; 128];
        for (dst, src) in out.as_chunks_mut::<32>().0.iter_mut().zip([
            &self.tag_out,
            &self.tag_in,
            &self.wrap_out,
            &self.wrap_in,
        ]) {
            dst.copy_from_slice(src);
        }
        out
    }

    pub fn from_bytes(bytes: &[u8; 128]) -> Self {
        let mut parts = bytes.as_chunks::<32>().0.iter().map(|c| {
            let mut k = [0u8; 32];
            k.copy_from_slice(c);
            k
        });
        let mut next = || parts.next().unwrap_or([0; 32]);
        Self {
            tag_out: next(),
            tag_in: next(),
            wrap_out: next(),
            wrap_in: next(),
        }
    }
}

fn tag(key: &[u8; 32], epoch: u32) -> [u8; 16] {
    let mut mac = <Hmac<Sha256> as hmac::KeyInit>::new_from_slice(key)
        .unwrap_or_else(|_| unreachable!("HMAC accepts any key length"));
    mac.update(b"VL/tag/v1");
    mac.update(&epoch.to_be_bytes());
    truncate16(&mac.finalize().into_bytes())
}

fn presence(key: &[u8; 32], handshake_hash: &[u8; 32]) -> [u8; 16] {
    let mut mac = <Hmac<Sha256> as hmac::KeyInit>::new_from_slice(key)
        .unwrap_or_else(|_| unreachable!("HMAC accepts any key length"));
    mac.update(b"VL/presence/v1");
    mac.update(handshake_hash);
    truncate16(&mac.finalize().into_bytes())
}

fn truncate16(bytes: &[u8]) -> [u8; 16] {
    let mut out = [0u8; 16];
    for (o, b) in out.iter_mut().zip(bytes) {
        *o = *b;
    }
    out
}

/// `Trunc16(SHA-256("VL/ack/v1" ‖ ack_secret))`, placed in the envelope header.
pub fn ack_commitment(ack_secret: &[u8; 16]) -> [u8; 16] {
    truncate16(
        &Sha256::new()
            .chain_update(b"VL/ack/v1")
            .chain_update(ack_secret)
            .finalize(),
    )
}

/// SHA-256 of `bytes` (attachment integrity).
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// Crockford Base32 alphabet: digits and letters without I, L, O, U.
const ID_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// A user's 32-character ID code: `Base32(Trunc20(SHA-256("VL/id/v1" ‖ fingerprint)))`.
/// 160 bits derived from the identity key, so it is unique without any
/// registry (a collision is as hard as finding a SHA-256 collision on 160
/// bits) and cannot be chosen or forged by anyone else. Display only; it is
/// never sent and never used as a key.
pub fn id_code(fingerprint: &[u8; 32]) -> String {
    let digest = Sha256::new()
        .chain_update(b"VL/id/v1")
        .chain_update(fingerprint)
        .finalize();
    let mut out = String::with_capacity(32);
    let mut acc: u32 = 0;
    let mut bits = 0;
    for &b in digest.iter().take(20) {
        acc = (acc << 8) | u32::from(b);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            if let Some(&c) = ID_ALPHABET.get(((acc >> bits) & 31) as usize) {
                out.push(char::from(c));
            }
        }
    }
    out
}

/// 60-digit safety number (12 groups of 5) that both contacts compute identically.
pub fn safety_number(fingerprint_a: &[u8; 32], fingerprint_b: &[u8; 32]) -> String {
    let (lo, hi) = if fingerprint_a < fingerprint_b {
        (fingerprint_a, fingerprint_b)
    } else {
        (fingerprint_b, fingerprint_a)
    };
    let digest = Sha512::new()
        .chain_update(b"VL/safety/v1")
        .chain_update(lo)
        .chain_update(hi)
        .finalize();
    digest
        .as_chunks::<5>()
        .0
        .iter()
        .take(12)
        .map(|c| {
            let v = c.iter().fold(0u64, |acc, b| (acc << 8) | u64::from(*b));
            format!("{:05}", v % 100_000)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Encrypts a padded inner message for `header`. The body is `nonce(12) ‖ ciphertext ‖ tag(16)`
/// and always exactly `header.body_len` bytes.
pub fn seal(key: &[u8; 32], header: &Header, padded_inner: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if padded_inner.len() + AEAD_OVERHEAD != usize::from(header.body_len) {
        return Err(CryptoError::Proto(vl_proto::ProtoError::BadLength));
    }
    let nonce: [u8; NONCE_LEN] = random_array()?;
    let cipher = ChaCha20Poly1305::new(&Key::from(*key));
    let aad = header.aad();
    let ct = cipher
        .encrypt(
            &Nonce::from(nonce),
            Payload {
                msg: padded_inner,
                aad: &aad,
            },
        )
        .map_err(|_| CryptoError::Aead)?;
    let mut body = Vec::with_capacity(NONCE_LEN + ct.len());
    body.extend_from_slice(&nonce);
    body.extend_from_slice(&ct);
    Ok(body)
}

/// Authenticates and decrypts an envelope body; any header change other than
/// `hop_count`/`copy_budget` causes failure.
pub fn open(key: &[u8; 32], header: &Header, body: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if body.len() < AEAD_OVERHEAD || body.len() != usize::from(header.body_len) {
        return Err(CryptoError::Aead);
    }
    let (nonce, ct) = body.split_at(NONCE_LEN);
    let mut n = [0u8; NONCE_LEN];
    n.copy_from_slice(nonce);
    let cipher = ChaCha20Poly1305::new(&Key::from(*key));
    let aad = header.aad();
    cipher
        .decrypt(&Nonce::from(n), Payload { msg: ct, aad: &aad })
        .map_err(|_| CryptoError::Aead)
}

#[cfg(test)]
mod tests {

    #[test]
    fn id_code_is_32_unambiguous_characters_and_differs_per_key() {
        let a = id_code(&[1; 32]);
        let b = id_code(&[2; 32]);
        assert_eq!(a.len(), 32);
        assert!(a.bytes().all(|c| ID_ALPHABET.contains(&c)));
        assert!(!a.contains(['I', 'L', 'O', 'U']));
        assert_eq!(a, id_code(&[1; 32]), "deterministic");
        assert_ne!(a, b);
        let codes: std::collections::HashSet<String> =
            (0..=255u8).map(|i| id_code(&[i; 32])).collect();
        assert_eq!(codes.len(), 256);
        // Golden value so the code never changes silently across versions.
        assert_eq!(a, "68QSTVZSN2PTTKR6XA2V77GH6JK0NPWF");
    }
    use super::*;
    use vl_proto::{EnvelopeKind, FLAG_VACCINE};

    fn keypair(seed: u8) -> (StaticSecret, [u8; 32]) {
        let s = StaticSecret::from([seed; 32]);
        let p = PublicKey::from(&s).to_bytes();
        (s, p)
    }

    fn header() -> Header {
        Header {
            kind: EnvelopeKind::Data,
            flags: FLAG_VACCINE,
            hop_count: 0,
            hop_limit: 8,
            copy_budget: 4,
            envelope_id: [1; 16],
            recipient_tag: [2; 16],
            expiry_min: 100,
            ack_commitment: [3; 16],
            body_len: 256,
        }
    }

    #[test]
    fn both_sides_derive_mirrored_keys() {
        let (a_sk, a_pk) = keypair(1);
        let (b_sk, b_pk) = keypair(2);
        let (ha, hb) = ([0xA; 32], [0xB; 32]);
        let a = PairKeys::derive(&a_sk, &ha, &b_pk, &hb).unwrap();
        let b = PairKeys::derive(&b_sk, &hb, &a_pk, &ha).unwrap();
        assert_eq!(a.tag_out(7), b.tag_in(7));
        assert_eq!(a.tag_in(7), b.tag_out(7));
        assert_eq!(a.wrap_out(), b.wrap_in());
        assert_ne!(a.tag_out(7), a.tag_in(7));
        assert_ne!(a.tag_out(7), a.tag_out(8));
        assert_eq!(PairKeys::from_bytes(&a.to_bytes()), a);
    }

    #[test]
    fn presence_tags_are_mirrored_directional_and_bound_to_the_link() {
        let (a_sk, a_pk) = keypair(1);
        let (b_sk, b_pk) = keypair(2);
        let (ha, hb) = ([0xA; 32], [0xB; 32]);
        let a = PairKeys::derive(&a_sk, &ha, &b_pk, &hb).unwrap();
        let b = PairKeys::derive(&b_sk, &hb, &a_pk, &ha).unwrap();
        let (h1, h2) = ([1; 32], [2; 32]);
        // Bob recognises the tag Alice shows him, and vice versa.
        assert_eq!(a.presence_out(&h1), b.presence_in(&h1));
        assert_eq!(b.presence_out(&h1), a.presence_in(&h1));
        // Directional: a tag can't be reflected back as the other side's.
        assert_ne!(a.presence_out(&h1), a.presence_in(&h1));
        // A new link gives unrelated tags, so sightings can't be linked.
        assert_ne!(a.presence_out(&h1), a.presence_out(&h2));
        // Known answer: Trunc16(HMAC-SHA256(key, "VL/presence/v1" ‖ h)), checked with Python's hmac.
        let kat = super::presence(&[1; 32], &[2; 32]);
        let hex: String = kat.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, "a5f20529d719aeabb9b1e2cb34e8e420");
    }

    #[test]
    fn rejects_self_pairing_and_low_order_points() {
        let (a_sk, _) = keypair(1);
        let (_, b_pk) = keypair(2);
        assert_eq!(
            PairKeys::derive(&a_sk, &[1; 32], &b_pk, &[1; 32]),
            Err(CryptoError::InvalidKey)
        );
        assert_eq!(
            PairKeys::derive(&a_sk, &[1; 32], &[0; 32], &[2; 32]),
            Err(CryptoError::NonContributory)
        );
    }

    #[test]
    fn seal_open_round_trip_and_aad_binding() {
        let key = [9u8; 32];
        let h = header();
        let inner = vec![0x42; 256 - AEAD_OVERHEAD];
        let body = seal(&key, &h, &inner).unwrap();
        assert_eq!(body.len(), 256);
        assert_eq!(open(&key, &h, &body).unwrap(), inner);

        let mut relayed = h.clone();
        relayed.hop_count = 3;
        relayed.copy_budget = 1;
        assert_eq!(open(&key, &relayed, &body).unwrap(), inner);

        let mut tampered = h.clone();
        tampered.expiry_min += 60;
        assert_eq!(open(&key, &tampered, &body), Err(CryptoError::Aead));

        let mut flipped = body.clone();
        flipped[100] ^= 1;
        assert_eq!(open(&key, &h, &flipped), Err(CryptoError::Aead));
        assert_eq!(open(&[8u8; 32], &h, &body), Err(CryptoError::Aead));
    }

    #[test]
    fn nonces_are_fresh() {
        let h = header();
        let inner = vec![0; 256 - AEAD_OVERHEAD];
        assert_ne!(
            seal(&[1; 32], &h, &inner).unwrap()[..12],
            seal(&[1; 32], &h, &inner).unwrap()[..12]
        );
    }

    #[test]
    fn safety_number_is_symmetric_and_sensitive() {
        let s = safety_number(&[1; 32], &[2; 32]);
        assert_eq!(s, safety_number(&[2; 32], &[1; 32]));
        assert_eq!(s.split(' ').count(), 12);
        assert!(
            s.split(' ')
                .all(|g| g.len() == 5 && g.chars().all(|c| c.is_ascii_digit()))
        );
        assert_ne!(s, safety_number(&[1; 32], &[3; 32]));
    }

    #[test]
    fn commitments_and_epochs() {
        assert_ne!(ack_commitment(&[1; 16]), ack_commitment(&[2; 16]));
        assert_eq!(epoch_of(0), 0);
        assert_eq!(epoch_of(EPOCH_SECONDS - 1), 0);
        assert_eq!(epoch_of(EPOCH_SECONDS), 1);
    }
}
