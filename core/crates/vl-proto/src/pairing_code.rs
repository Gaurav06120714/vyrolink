//! Text form of a pairing card for QR codes: `VL1:` followed by Base45
//! (RFC 9285). Base45 uses exactly the QR alphanumeric character set, so the
//! code fits a smaller QR symbol than Base64 and survives scanners that only
//! return text.

use crate::ProtoError;

pub const PAIRING_CODE_PREFIX: &str = "VL1:";
/// Longest code accepted; checked before any allocation.
pub const MAX_PAIRING_CODE_LEN: usize = 4096;

const ALPHABET: &[u8; 45] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

pub fn encode_pairing_code(card: &[u8]) -> String {
    let mut out = String::with_capacity(PAIRING_CODE_PREFIX.len() + card.len() * 3 / 2 + 2);
    out.push_str(PAIRING_CODE_PREFIX);
    for pair in card.chunks(2) {
        let (mut n, digits) = match pair {
            [a, b] => (usize::from(*a) * 256 + usize::from(*b), 3),
            [a] => (usize::from(*a), 2),
            _ => continue,
        };
        for _ in 0..digits {
            if let Some(&c) = ALPHABET.get(n % 45) {
                out.push(char::from(c));
            }
            n /= 45;
        }
    }
    out
}

pub fn decode_pairing_code(code: &str) -> Result<Vec<u8>, ProtoError> {
    if code.len() > MAX_PAIRING_CODE_LEN {
        return Err(ProtoError::BadLength);
    }
    let body = code
        .strip_prefix(PAIRING_CODE_PREFIX)
        .ok_or(ProtoError::InvalidField("pairing code prefix"))?;
    let values = body
        .bytes()
        .map(|c| ALPHABET.iter().position(|&a| a == c))
        .collect::<Option<Vec<usize>>>()
        .ok_or(ProtoError::InvalidField("pairing code character"))?;
    let mut out = Vec::with_capacity(values.len() * 2 / 3 + 1);
    for group in values.chunks(3) {
        match group {
            [c, d, e] => {
                let n = c + d * 45 + e * 2025;
                let n = u16::try_from(n).map_err(|_| ProtoError::InvalidField("base45 group"))?;
                out.extend_from_slice(&n.to_be_bytes());
            }
            [c, d] => {
                let n = u8::try_from(c + d * 45)
                    .map_err(|_| ProtoError::InvalidField("base45 group"))?;
                out.push(n);
            }
            _ => return Err(ProtoError::Truncated),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b45(s: &str) -> String {
        encode_pairing_code(s.as_bytes())[PAIRING_CODE_PREFIX.len()..].to_owned()
    }

    #[test]
    fn rfc_9285_examples() {
        assert_eq!(b45("AB"), "BB8");
        assert_eq!(b45("Hello!!"), "%69 VD92EX0");
        assert_eq!(b45("base-45"), "UJCLQE7W581");
        assert_eq!(b45("ietf!"), "QED8WEX0");
        assert_eq!(
            decode_pairing_code("VL1:QED8WEX0").unwrap(),
            b"ietf!".to_vec()
        );
    }

    #[test]
    fn round_trips_every_length() {
        for len in 0..64usize {
            let bytes: Vec<u8> = (0..len).map(|i| (i * 37 + 255) as u8).collect();
            assert_eq!(
                decode_pairing_code(&encode_pairing_code(&bytes)).unwrap(),
                bytes
            );
        }
    }

    #[test]
    fn rejects_foreign_and_malformed_codes() {
        assert!(decode_pairing_code("https://example.com").is_err());
        assert!(decode_pairing_code("VL1:abc").is_err());
        assert!(decode_pairing_code("VL1:A").is_err());
        assert!(decode_pairing_code("VL1:GGW").is_err());
        assert!(decode_pairing_code("VL1:::").is_err());
        let huge = format!("VL1:{}", "0".repeat(MAX_PAIRING_CODE_LEN));
        assert_eq!(decode_pairing_code(&huge), Err(ProtoError::BadLength));
    }
}
