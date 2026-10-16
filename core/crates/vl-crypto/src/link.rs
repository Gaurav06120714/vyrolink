use crate::CryptoError;
use std::fmt;

/// Anonymous link encryption between neighbouring devices (ADR-008). No static
/// keys: it hides relay metadata from passive observers but does not
/// authenticate the peer.
pub const NOISE_PATTERN: &str = "Noise_NN_25519_ChaChaPoly_SHA256";
const PROLOGUE: &[u8] = b"VL/link/v1";
const MAX_NOISE_MSG: usize = 65_535;

pub struct LinkHandshake(snow::HandshakeState);

impl fmt::Debug for LinkHandshake {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LinkHandshake")
    }
}

fn noise_err(e: snow::Error) -> CryptoError {
    CryptoError::Noise(e.to_string())
}

impl LinkHandshake {
    fn build(initiator: bool) -> Result<Self, CryptoError> {
        let params = NOISE_PATTERN.parse().map_err(noise_err)?;
        let builder = snow::Builder::new(params)
            .prologue(PROLOGUE)
            .map_err(noise_err)?;
        let state = if initiator {
            builder.build_initiator()
        } else {
            builder.build_responder()
        };
        state.map(Self).map_err(noise_err)
    }

    pub fn initiator() -> Result<Self, CryptoError> {
        Self::build(true)
    }

    pub fn responder() -> Result<Self, CryptoError> {
        Self::build(false)
    }

    pub fn write_message(&mut self) -> Result<Vec<u8>, CryptoError> {
        let mut buf = vec![0u8; MAX_NOISE_MSG];
        let n = self.0.write_message(&[], &mut buf).map_err(noise_err)?;
        buf.truncate(n);
        Ok(buf)
    }

    pub fn read_message(&mut self, msg: &[u8]) -> Result<(), CryptoError> {
        let mut buf = vec![0u8; MAX_NOISE_MSG];
        let n = self.0.read_message(msg, &mut buf).map_err(noise_err)?;
        if n != 0 {
            return Err(CryptoError::Noise("unexpected handshake payload".into()));
        }
        Ok(())
    }

    pub fn is_finished(&self) -> bool {
        self.0.is_handshake_finished()
    }

    pub fn into_transport(self) -> Result<LinkTransport, CryptoError> {
        self.0
            .into_transport_mode()
            .map(LinkTransport)
            .map_err(noise_err)
    }
}

pub struct LinkTransport(snow::TransportState);

impl fmt::Debug for LinkTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LinkTransport")
    }
}

impl LinkTransport {
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let mut buf = vec![0u8; plaintext.len() + 16];
        let n = self
            .0
            .write_message(plaintext, &mut buf)
            .map_err(noise_err)?;
        buf.truncate(n);
        Ok(buf)
    }

    /// Fails on tampering, reordering or replay (Noise nonces are strictly sequential).
    pub fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let mut buf = vec![0u8; ciphertext.len()];
        let n = self
            .0
            .read_message(ciphertext, &mut buf)
            .map_err(noise_err)?;
        buf.truncate(n);
        Ok(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connect() -> (LinkTransport, LinkTransport) {
        let mut i = LinkHandshake::initiator().unwrap();
        let mut r = LinkHandshake::responder().unwrap();
        r.read_message(&i.write_message().unwrap()).unwrap();
        i.read_message(&r.write_message().unwrap()).unwrap();
        assert!(i.is_finished() && r.is_finished());
        (i.into_transport().unwrap(), r.into_transport().unwrap())
    }

    #[test]
    fn two_message_handshake_then_bidirectional_transport() {
        let (mut i, mut r) = connect();
        let c = i.encrypt(b"summary").unwrap();
        assert_eq!(r.decrypt(&c).unwrap(), b"summary");
        let d = r.encrypt(b"request").unwrap();
        assert_eq!(i.decrypt(&d).unwrap(), b"request");
    }

    #[test]
    fn replay_and_tamper_fail() {
        let (mut i, mut r) = connect();
        let c1 = i.encrypt(b"one").unwrap();
        r.decrypt(&c1).unwrap();
        assert!(r.decrypt(&c1).is_err());
        let (mut i2, mut r2) = connect();
        let mut c = i2.encrypt(b"two").unwrap();
        c[0] ^= 1;
        assert!(r2.decrypt(&c).is_err());
    }
}
