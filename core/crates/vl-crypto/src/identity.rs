use crate::{CryptoError, PairKeys, random_array};
use sha2::{Digest, Sha256};
use std::fmt;
use vl_proto::{MAX_OTKS, PairingCard};
use vodozemac::olm::{
    Account, AccountPickle, InboundCreationResult, PreKeyMessage, Session, SessionConfig,
};
use vodozemac::{Curve25519PublicKey, Ed25519PublicKey, Ed25519Signature};
use x25519_dalek::{PublicKey, StaticSecret};

/// Olm session format. Version 1 is the standard (libolm-compatible) format;
/// its 8-byte truncated MAC is backed by the 16-byte ChaCha20-Poly1305 tag of
/// the envelope wrap that always encloses it.
pub(crate) const SESSION_CONFIG: SessionConfig = SessionConfig::version_1();

/// This device's long-term keys: the Olm account (Curve25519 identity,
/// Ed25519 fingerprint, one-time keys) and the separate X25519 addressing key
/// used only for pairwise key derivation (no cross-protocol key reuse).
pub struct Identity {
    account: Account,
    addressing: StaticSecret,
}

impl fmt::Debug for Identity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Identity")
            .field("fingerprint", &hex8(&self.fingerprint_key()))
            .finish_non_exhaustive()
    }
}

fn hex8(b: &[u8]) -> String {
    b.iter().take(8).map(|x| format!("{x:02x}")).collect()
}

/// SHA-256 of a card's full encoding; binds pairwise keys to both exchanged cards.
pub fn card_hash(card: &PairingCard) -> Result<[u8; 32], CryptoError> {
    Ok(Sha256::digest(card.encode()?).into())
}

/// True for X25519 points in the small subgroup (including non-canonical
/// encodings). Clamped scalars are multiples of 8, so the product is the
/// identity exactly when the point has no prime-order component.
fn is_low_order(point: &[u8; 32]) -> bool {
    !StaticSecret::from([0x55; 32])
        .diffie_hellman(&PublicKey::from(*point))
        .was_contributory()
}

/// Verifies a card's Ed25519 signature and rejects low-order X25519 keys, so a
/// contact cannot force predictable Olm or pairwise secrets with a self-signed card.
pub fn verify_card(card: &PairingCard) -> Result<(), CryptoError> {
    if is_low_order(&card.olm_identity_key)
        || is_low_order(&card.addressing_key)
        || card.one_time_keys.iter().any(is_low_order)
    {
        return Err(CryptoError::InvalidKey);
    }
    let key = Ed25519PublicKey::from_slice(&card.olm_fingerprint_key)
        .map_err(|_| CryptoError::InvalidKey)?;
    let sig =
        Ed25519Signature::from_slice(&card.signature).map_err(|_| CryptoError::BadSignature)?;
    key.verify(&card.signed_bytes()?, &sig)
        .map_err(|_| CryptoError::BadSignature)
}

impl Identity {
    pub fn generate() -> Result<Self, CryptoError> {
        Ok(Self {
            account: Account::new(),
            addressing: StaticSecret::from(random_array::<32>()?),
        })
    }

    pub fn identity_key(&self) -> [u8; 32] {
        self.account.curve25519_key().to_bytes()
    }

    pub fn fingerprint_key(&self) -> [u8; 32] {
        *self.account.ed25519_key().as_bytes()
    }

    pub fn addressing_public(&self) -> [u8; 32] {
        PublicKey::from(&self.addressing).to_bytes()
    }

    /// Generates `otk_count` fresh one-time keys and returns a signed card carrying them.
    pub fn create_card(
        &mut self,
        display_name: &str,
        otk_count: usize,
    ) -> Result<PairingCard, CryptoError> {
        if otk_count == 0 || otk_count > MAX_OTKS {
            return Err(CryptoError::InvalidKey);
        }
        let generated = self.account.generate_one_time_keys(otk_count);
        let one_time_keys = generated.created.iter().map(|k| k.to_bytes()).collect();
        self.account.mark_keys_as_published();
        let mut card = PairingCard {
            olm_identity_key: self.identity_key(),
            olm_fingerprint_key: self.fingerprint_key(),
            addressing_key: self.addressing_public(),
            one_time_keys,
            nonce: random_array()?,
            display_name: display_name.to_owned(),
            signature: [0; 64],
        };
        card.signature = self.account.sign(card.signed_bytes()?).to_bytes();
        Ok(card)
    }

    /// Verifies `theirs` and derives the directional pairwise keys.
    pub fn pair(&self, mine: &PairingCard, theirs: &PairingCard) -> Result<PairKeys, CryptoError> {
        verify_card(theirs)?;
        if mine.olm_fingerprint_key != self.fingerprint_key() {
            return Err(CryptoError::InvalidKey);
        }
        PairKeys::derive(
            &self.addressing,
            &card_hash(mine)?,
            &theirs.addressing_key,
            &card_hash(theirs)?,
        )
    }

    /// Starts an Olm session to a contact using one of the one-time keys from their card.
    pub fn outbound_session(
        &self,
        theirs: &PairingCard,
        otk_index: usize,
    ) -> Result<Session, CryptoError> {
        let otk = theirs
            .one_time_keys
            .get(otk_index)
            .ok_or(CryptoError::InvalidKey)?;
        self.account
            .create_outbound_session(
                SESSION_CONFIG,
                Curve25519PublicKey::from_bytes(theirs.olm_identity_key),
                Curve25519PublicKey::from_bytes(*otk),
            )
            .map_err(|e| CryptoError::Olm(e.to_string()))
    }

    /// Creates a session from a contact's first (pre-key) message. The one-time key it
    /// used is consumed by vodozemac, so replaying the message cannot recreate the session.
    pub fn inbound_session(
        &mut self,
        their_identity_key: &[u8; 32],
        message: &PreKeyMessage,
    ) -> Result<(Session, Vec<u8>), CryptoError> {
        let InboundCreationResult { session, plaintext } = self
            .account
            .create_inbound_session(
                SESSION_CONFIG,
                Curve25519PublicKey::from_bytes(*their_identity_key),
                message,
            )
            .map_err(|e| CryptoError::Olm(e.to_string()))?;
        Ok((session, plaintext))
    }

    /// Encrypted state for the Class C store. `pickle_key` comes from the platform key store.
    pub fn to_pickle(&self, pickle_key: &[u8; 32]) -> (String, [u8; 32]) {
        (
            self.account.pickle().encrypt(pickle_key),
            self.addressing.to_bytes(),
        )
    }

    pub fn from_pickle(
        account: &str,
        addressing: [u8; 32],
        pickle_key: &[u8; 32],
    ) -> Result<Self, CryptoError> {
        let pickle =
            AccountPickle::from_encrypted(account, pickle_key).map_err(|_| CryptoError::Pickle)?;
        Ok(Self {
            account: Account::from_pickle(pickle),
            addressing: StaticSecret::from(addressing),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_signature_verifies_and_detects_tampering() {
        let mut id = Identity::generate().unwrap();
        let card = id.create_card("Asha", 5).unwrap();
        assert_eq!(card.one_time_keys.len(), 5);
        verify_card(&card).unwrap();
        let mut renamed = card.clone();
        renamed.display_name = "Mallory".into();
        assert_eq!(verify_card(&renamed), Err(CryptoError::BadSignature));
        let mut swapped = card.clone();
        swapped.addressing_key = [7; 32];
        assert_eq!(verify_card(&swapped), Err(CryptoError::BadSignature));
        let decoded = PairingCard::decode(&card.encode().unwrap()).unwrap();
        verify_card(&decoded).unwrap();
    }

    #[test]
    fn mutual_pairing_produces_mirrored_keys() {
        let (mut a, mut b) = (Identity::generate().unwrap(), Identity::generate().unwrap());
        let (ca, cb) = (
            a.create_card("A", 3).unwrap(),
            b.create_card("B", 3).unwrap(),
        );
        let ka = a.pair(&ca, &cb).unwrap();
        let kb = b.pair(&cb, &ca).unwrap();
        assert_eq!(ka.tag_out(1), kb.tag_in(1));
        assert_eq!(ka.wrap_out(), kb.wrap_in());
        assert!(a.pair(&cb, &ca).is_err());
    }

    // Regression for the February 2026 vodozemac report (all-zero DH outputs): a
    // malicious contact signs a valid card around low-order X25519 points.
    #[test]
    fn low_order_keys_in_a_signed_card_are_rejected() {
        let alice = Identity::generate().unwrap();
        let mut alice_owned = Identity::generate().unwrap();
        let alice_card = alice_owned.create_card("A", 1).unwrap();
        let mut mallory = Identity::generate().unwrap();
        let low_order: [[u8; 32]; 2] = [[0; 32], {
            let mut p = [0u8; 32];
            p[0] = 1;
            p
        }];
        for point in low_order {
            let mut card = mallory.create_card("M", 1).unwrap();
            card.one_time_keys[0] = point;
            card.signature = mallory
                .account
                .sign(card.signed_bytes().unwrap())
                .to_bytes();
            assert_eq!(verify_card(&card), Err(CryptoError::InvalidKey));
            // vodozemac 0.11.1 also refuses, independently of our card check.
            assert!(alice.outbound_session(&card, 0).is_err());

            let mut card = mallory.create_card("M", 1).unwrap();
            card.olm_identity_key = point;
            card.signature = mallory
                .account
                .sign(card.signed_bytes().unwrap())
                .to_bytes();
            assert_eq!(verify_card(&card), Err(CryptoError::InvalidKey));
            assert!(alice.outbound_session(&card, 0).is_err());

            let mut card = mallory.create_card("M", 1).unwrap();
            card.addressing_key = point;
            card.signature = mallory
                .account
                .sign(card.signed_bytes().unwrap())
                .to_bytes();
            assert_eq!(
                alice_owned.pair(&alice_card, &card).err(),
                Some(CryptoError::InvalidKey)
            );
            assert!(!is_low_order(&alice_card.addressing_key));
        }
    }

    #[test]
    fn pickle_round_trip_keeps_keys() {
        let mut id = Identity::generate().unwrap();
        let card = id.create_card("A", 2).unwrap();
        let key = [5u8; 32];
        let (acct, ak) = id.to_pickle(&key);
        let restored = Identity::from_pickle(&acct, ak, &key).unwrap();
        assert_eq!(restored.fingerprint_key(), card.olm_fingerprint_key);
        assert_eq!(restored.addressing_public(), card.addressing_key);
        assert_eq!(
            Identity::from_pickle(&acct, ak, &[6u8; 32]).err(),
            Some(CryptoError::Pickle)
        );
    }
}
