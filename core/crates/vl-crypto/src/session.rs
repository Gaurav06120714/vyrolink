use crate::{CryptoError, Identity};
use vodozemac::olm::{OlmMessage, Session};

/// An Olm message as carried in an inner `TEXT`/`RECEIPT_READ` payload:
/// `type(u8: 0 = pre-key, 1 = normal) ‖ olm bytes`.
#[derive(Debug, Clone)]
pub struct OlmPayload(OlmMessage);

impl OlmPayload {
    pub fn encode(&self) -> Vec<u8> {
        let (kind, bytes) = self.0.to_parts();
        let mut out = Vec::with_capacity(1 + bytes.len());
        out.push(u8::try_from(kind).unwrap_or(u8::MAX));
        out.extend_from_slice(&bytes);
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, CryptoError> {
        let (kind, rest) = bytes
            .split_first()
            .ok_or(CryptoError::Olm("empty payload".into()))?;
        OlmMessage::from_parts(usize::from(*kind), rest)
            .map(Self)
            .map_err(|e| CryptoError::Olm(e.to_string()))
    }

    pub fn is_pre_key(&self) -> bool {
        matches!(self.0, OlmMessage::PreKey(_))
    }
}

pub fn encrypt_olm(session: &mut Session, plaintext: &[u8]) -> Result<OlmPayload, CryptoError> {
    session
        .encrypt(plaintext)
        .map(OlmPayload)
        .map_err(|e| CryptoError::Olm(e.to_string()))
}

/// Decrypts with the existing session if there is one; otherwise creates an
/// inbound session from a pre-key message. Returns the session to persist.
pub fn decrypt_olm(
    identity: &mut Identity,
    their_identity_key: &[u8; 32],
    session: Option<Session>,
    payload: &OlmPayload,
) -> Result<(Session, Vec<u8>), CryptoError> {
    if let Some(mut s) = session {
        let plain = s
            .decrypt(&payload.0)
            .map_err(|e| CryptoError::Olm(e.to_string()))?;
        return Ok((s, plain));
    }
    match &payload.0 {
        OlmMessage::PreKey(m) => identity.inbound_session(their_identity_key, m),
        OlmMessage::Normal(_) => Err(CryptoError::Olm("no session for normal message".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair() -> (Identity, Identity, vl_proto::PairingCard) {
        let a = Identity::generate().unwrap();
        let mut b = Identity::generate().unwrap();
        let cb = b.create_card("B", 2).unwrap();
        (a, b, cb)
    }

    #[test]
    fn first_message_creates_session_then_ratchets() {
        let (a, mut b, cb) = pair();
        let mut out = a.outbound_session(&cb, 0).unwrap();
        let m1 = encrypt_olm(&mut out, b"hello").unwrap();
        assert!(m1.is_pre_key());
        let wire = OlmPayload::decode(&m1.encode()).unwrap();
        let (mut b_session, plain) = decrypt_olm(&mut b, &a.identity_key(), None, &wire).unwrap();
        assert_eq!(plain, b"hello");

        let reply = encrypt_olm(&mut b_session, b"hi back").unwrap();
        assert!(!reply.is_pre_key());
        assert_eq!(out.decrypt(&reply.0).unwrap(), b"hi back");
    }

    #[test]
    fn replayed_pre_key_cannot_recreate_session() {
        let (a, mut b, cb) = pair();
        let mut out = a.outbound_session(&cb, 0).unwrap();
        let m1 = encrypt_olm(&mut out, b"x").unwrap();
        decrypt_olm(&mut b, &a.identity_key(), None, &m1).unwrap();
        assert!(decrypt_olm(&mut b, &a.identity_key(), None, &m1).is_err());
    }

    #[test]
    fn wrong_sender_identity_is_rejected() {
        let (a, mut b, cb) = pair();
        let mallory = Identity::generate().unwrap();
        let mut out = a.outbound_session(&cb, 0).unwrap();
        let m1 = encrypt_olm(&mut out, b"x").unwrap();
        assert!(decrypt_olm(&mut b, &mallory.identity_key(), None, &m1).is_err());
    }

    #[test]
    fn out_of_order_within_window_and_replay_rejected() {
        let (a, mut b, cb) = pair();
        let mut out = a.outbound_session(&cb, 0).unwrap();
        let first = encrypt_olm(&mut out, b"0").unwrap();
        let (mut bs, _) = decrypt_olm(&mut b, &a.identity_key(), None, &first).unwrap();
        let reply = encrypt_olm(&mut bs, b"ack").unwrap();
        out.decrypt(&reply.0).unwrap();
        let msgs: Vec<_> = (1..=10)
            .map(|i| encrypt_olm(&mut out, format!("{i}").as_bytes()).unwrap())
            .collect();
        for m in msgs.iter().rev() {
            bs.decrypt(&m.0).unwrap();
        }
        assert!(bs.decrypt(&msgs[0].0).is_err(), "replay must fail");
    }
}
