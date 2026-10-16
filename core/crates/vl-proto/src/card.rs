use crate::{ProtoError, Reader};

pub const CARD_VERSION: u8 = 1;
pub const MAX_OTKS: usize = 8;
pub const MAX_NAME_LEN: usize = 32;
const SIGNATURE_LEN: usize = 64;

/// Identity material exchanged in person by QR code (Doc 06 §3.2).
/// The signature covers [`PairingCard::signed_bytes`], a canonical encoding of
/// every other field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingCard {
    pub olm_identity_key: [u8; 32],
    pub olm_fingerprint_key: [u8; 32],
    pub addressing_key: [u8; 32],
    pub one_time_keys: Vec<[u8; 32]>,
    pub nonce: [u8; 16],
    pub display_name: String,
    pub signature: [u8; SIGNATURE_LEN],
}

impl PairingCard {
    /// Canonical bytes covered by the signature.
    pub fn signed_bytes(&self) -> Result<Vec<u8>, ProtoError> {
        if self.one_time_keys.is_empty() || self.one_time_keys.len() > MAX_OTKS {
            return Err(ProtoError::InvalidField("one_time_keys"));
        }
        let name = self.display_name.as_bytes();
        if name.len() > MAX_NAME_LEN {
            return Err(ProtoError::InvalidField("display_name"));
        }
        let mut out =
            Vec::with_capacity(1 + 96 + 1 + 32 * self.one_time_keys.len() + 16 + 1 + name.len());
        out.push(CARD_VERSION);
        out.extend_from_slice(&self.olm_identity_key);
        out.extend_from_slice(&self.olm_fingerprint_key);
        out.extend_from_slice(&self.addressing_key);
        out.push(self.one_time_keys.len() as u8);
        for k in &self.one_time_keys {
            out.extend_from_slice(k);
        }
        out.extend_from_slice(&self.nonce);
        out.push(name.len() as u8);
        out.extend_from_slice(name);
        Ok(out)
    }

    pub fn encode(&self) -> Result<Vec<u8>, ProtoError> {
        let mut out = self.signed_bytes()?;
        out.extend_from_slice(&self.signature);
        Ok(out)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ProtoError> {
        let mut r = Reader::new(bytes);
        let version = r.u8()?;
        if version != CARD_VERSION {
            return Err(ProtoError::UnsupportedVersion(version));
        }
        let olm_identity_key = r.array()?;
        let olm_fingerprint_key = r.array()?;
        let addressing_key = r.array()?;
        let n = usize::from(r.u8()?);
        if n == 0 || n > MAX_OTKS {
            return Err(ProtoError::InvalidField("one_time_keys"));
        }
        let mut one_time_keys = Vec::with_capacity(n);
        for _ in 0..n {
            one_time_keys.push(r.array()?);
        }
        let nonce = r.array()?;
        let name_len = usize::from(r.u8()?);
        if name_len > MAX_NAME_LEN {
            return Err(ProtoError::InvalidField("display_name"));
        }
        let display_name = std::str::from_utf8(r.take(name_len)?)
            .map_err(|_| ProtoError::InvalidField("display_name"))?
            .to_owned();
        let signature = r.array()?;
        r.finish()?;
        Ok(Self {
            olm_identity_key,
            olm_fingerprint_key,
            addressing_key,
            one_time_keys,
            nonce,
            display_name,
            signature,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card() -> PairingCard {
        PairingCard {
            olm_identity_key: [1; 32],
            olm_fingerprint_key: [2; 32],
            addressing_key: [3; 32],
            one_time_keys: vec![[4; 32]; 5],
            nonce: [5; 16],
            display_name: "Asha".into(),
            signature: [6; 64],
        }
    }

    #[test]
    fn round_trip_and_size() {
        let c = card();
        let bytes = c.encode().unwrap();
        assert_eq!(bytes.len(), 1 + 96 + 1 + 160 + 16 + 1 + 4 + 64);
        assert_eq!(PairingCard::decode(&bytes).unwrap(), c);
    }

    #[test]
    fn rejects_bad_cards() {
        let mut c = card();
        c.display_name = "x".repeat(33);
        assert!(c.encode().is_err());
        let mut bytes = card().encode().unwrap();
        bytes[0] = 9;
        assert_eq!(
            PairingCard::decode(&bytes),
            Err(ProtoError::UnsupportedVersion(9))
        );
        let ok = card().encode().unwrap();
        assert_eq!(
            PairingCard::decode(&ok[..ok.len() - 1]),
            Err(ProtoError::Truncated)
        );
    }
}
