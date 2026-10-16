use crate::{
    CryptoError, OlmPayload, PairKeys, ack_commitment, encrypt_olm, epoch_of, open, random_array,
    seal,
};
use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, KeyInit, Nonce};
use vl_proto::{
    AEAD_OVERHEAD, ChunkPayload, Content, DEFAULT_HOP_LIMIT, Envelope, EnvelopeKind, FLAG_VACCINE,
    Header, INNER_HEADER_LEN, Inner, InnerKind, Receipt, bucket_for,
};
use vodozemac::olm::Session;

/// Expiry jitter so the coarse expiry does not reveal the exact send time (Doc 07 §3.1).
const EXPIRY_JITTER_MIN: u32 = 30;

#[derive(Debug, Clone)]
pub struct Outgoing {
    pub envelope: Envelope,
    pub message_id: [u8; 16],
    pub ack_secret: [u8; 16],
    /// The sender timestamp sealed inside the message.
    pub sent_at_ms: u64,
}

/// Parameters chosen by the routing policy for a new envelope.
#[derive(Debug, Clone, Copy)]
pub struct SendParams {
    pub now_secs: u64,
    /// Sender timestamp carried in the inner message: the node's hybrid logical
    /// clock, so causally later events never carry a smaller time.
    pub sent_at_ms: u64,
    pub lifetime_min: u32,
    pub copy_budget: u8,
}

fn expiry(now_secs: u64, lifetime_min: u32) -> Result<u32, CryptoError> {
    let now_min = u32::try_from(now_secs / 60).map_err(|_| CryptoError::InvalidKey)?;
    let jitter = u32::from(random_array::<1>()?[0]) % (2 * EXPIRY_JITTER_MIN + 1);
    let target = now_min
        .saturating_add(lifetime_min)
        .saturating_add(jitter)
        .saturating_sub(EXPIRY_JITTER_MIN);
    Ok(target.max(now_min.saturating_add(1)))
}

fn seal_inner(
    pair: &PairKeys,
    kind: EnvelopeKind,
    vaccine: Option<[u8; 16]>,
    inner: &Inner,
    p: SendParams,
) -> Result<Envelope, CryptoError> {
    let needed = INNER_HEADER_LEN + inner.payload.len() + AEAD_OVERHEAD;
    let body_len = bucket_for(needed).ok_or(CryptoError::Proto(vl_proto::ProtoError::BadLength))?;
    let header = Header {
        kind,
        flags: if vaccine.is_some() { FLAG_VACCINE } else { 0 },
        hop_count: 0,
        hop_limit: DEFAULT_HOP_LIMIT,
        copy_budget: p.copy_budget,
        envelope_id: random_array()?,
        recipient_tag: pair.tag_out(epoch_of(p.now_secs)),
        expiry_min: expiry(p.now_secs, p.lifetime_min)?,
        ack_commitment: vaccine.map(|s| ack_commitment(&s)).unwrap_or([0; 16]),
        body_len,
    };
    let padded = inner.encode_padded(usize::from(body_len) - AEAD_OVERHEAD)?;
    let body = seal(pair.wrap_out(), &header, &padded)?;
    Ok(Envelope { header, body })
}

/// Encrypts `text` with Olm and wraps it for the contact. Fails if it does not fit the largest bucket.
pub fn build_text(
    pair: &PairKeys,
    session: &mut Session,
    text: &str,
    p: SendParams,
) -> Result<Outgoing, CryptoError> {
    build_olm(
        pair,
        session,
        InnerKind::Text,
        text.as_bytes(),
        p,
        random_array()?,
    )
}

/// Encrypts a conversation [`Content`] event with Olm and wraps it (inner kind `CONTENT`).
pub fn build_content(
    pair: &PairKeys,
    session: &mut Session,
    content: &Content,
    p: SendParams,
) -> Result<Outgoing, CryptoError> {
    build_content_bytes(pair, session, &content.encode()?, p)
}

/// Like [`build_content`] for already-encoded content (e.g. a content type this
/// build does not know, to exercise forward compatibility).
pub fn build_content_bytes(
    pair: &PairKeys,
    session: &mut Session,
    encoded: &[u8],
    p: SendParams,
) -> Result<Outgoing, CryptoError> {
    build_olm(
        pair,
        session,
        InnerKind::Content,
        encoded,
        p,
        random_array()?,
    )
}

/// Like [`build_content_bytes`] but keeps a given message id: a resend of an
/// expired message, so a recipient that already has it deduplicates and still
/// returns a receipt for the same id.
pub fn build_content_bytes_with_id(
    pair: &PairKeys,
    session: &mut Session,
    encoded: &[u8],
    p: SendParams,
    message_id: [u8; 16],
) -> Result<Outgoing, CryptoError> {
    build_olm(pair, session, InnerKind::Content, encoded, p, message_id)
}

fn build_olm(
    pair: &PairKeys,
    session: &mut Session,
    kind: InnerKind,
    plaintext: &[u8],
    p: SendParams,
    message_id: [u8; 16],
) -> Result<Outgoing, CryptoError> {
    let ack_secret = random_array()?;
    let inner = Inner {
        kind,
        message_id,
        ack_secret,
        sent_at_ms: p.sent_at_ms,
        payload: encrypt_olm(session, plaintext)?.encode(),
    };
    let envelope = seal_inner(pair, EnvelopeKind::Data, Some(ack_secret), &inner, p)?;
    Ok(Outgoing {
        envelope,
        message_id,
        ack_secret,
        sent_at_ms: p.sent_at_ms,
    })
}

fn chunk_nonce_aad(attachment_id: &[u8; 16], index: u16) -> ([u8; 12], Vec<u8>) {
    let mut nonce = [0u8; 12];
    nonce[10..].copy_from_slice(&index.to_be_bytes());
    let mut aad = b"VL/att/v1".to_vec();
    aad.extend_from_slice(attachment_id);
    aad.extend_from_slice(&index.to_be_bytes());
    (nonce, aad)
}

/// Seals one attachment chunk with the attachment's one-time key (ADR-026).
/// ChaCha20-Poly1305 (RFC 8439); the key is random and used for one
/// attachment only, so the chunk index is a unique nonce. The AAD binds the
/// attachment id and index, so chunks cannot be moved between attachments
/// or positions.
pub fn seal_chunk(
    key: &[u8; 32],
    attachment_id: &[u8; 16],
    index: u16,
    data: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let (nonce, aad) = chunk_nonce_aad(attachment_id, index);
    ChaCha20Poly1305::new(&Key::from(*key))
        .encrypt(
            &Nonce::from(nonce),
            Payload {
                msg: data,
                aad: &aad,
            },
        )
        .map_err(|_| CryptoError::Aead)
}

pub fn open_chunk(
    key: &[u8; 32],
    attachment_id: &[u8; 16],
    index: u16,
    ciphertext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let (nonce, aad) = chunk_nonce_aad(attachment_id, index);
    ChaCha20Poly1305::new(&Key::from(*key))
        .decrypt(
            &Nonce::from(nonce),
            Payload {
                msg: ciphertext,
                aad: &aad,
            },
        )
        .map_err(|_| CryptoError::Aead)
}

/// Builds an `AttachmentChunk` envelope: the sealed chunk inside the normal
/// pairwise wrap, with its own message id and delivery vaccine.
pub fn build_attachment_chunk(
    pair: &PairKeys,
    chunk: &ChunkPayload,
    p: SendParams,
) -> Result<Outgoing, CryptoError> {
    let message_id = random_array()?;
    let ack_secret = random_array()?;
    let inner = Inner {
        kind: InnerKind::AttachmentChunk,
        message_id,
        ack_secret,
        sent_at_ms: p.sent_at_ms,
        payload: chunk.encode()?,
    };
    let envelope = seal_inner(pair, EnvelopeKind::Data, Some(ack_secret), &inner, p)?;
    Ok(Outgoing {
        envelope,
        message_id,
        ack_secret,
        sent_at_ms: p.sent_at_ms,
    })
}

/// Builds a wrap-only receipt (works while the device is locked). Receipts carry no vaccine.
pub fn build_receipt(
    pair: &PairKeys,
    kind: InnerKind,
    refs: &[[u8; 16]],
    p: SendParams,
) -> Result<Envelope, CryptoError> {
    if !matches!(
        kind,
        InnerKind::ReceiptDelivered | InnerKind::ReceiptUndecryptable
    ) {
        return Err(CryptoError::InvalidKey);
    }
    let inner = Inner {
        kind,
        message_id: random_array()?,
        ack_secret: [0; 16],
        sent_at_ms: p.sent_at_ms,
        payload: Receipt {
            refs: refs.to_vec(),
            at_ms: p.now_secs.saturating_mul(1000),
        }
        .encode()?,
    };
    seal_inner(pair, EnvelopeKind::Receipt, None, &inner, p)
}

/// What the recipient learns after unwrapping (before any Olm decryption).
#[derive(Debug, Clone)]
pub struct Opened {
    pub inner: Inner,
    /// The vaccine to release, present only if the sender committed to it and the commitment matches.
    pub vaccine: Option<[u8; 16]>,
}

/// Unwraps an envelope addressed to this device by a known contact.
pub fn open_from(pair: &PairKeys, envelope: &Envelope) -> Result<Opened, CryptoError> {
    let plain = open(pair.wrap_in(), &envelope.header, &envelope.body)?;
    let inner = Inner::decode(&plain)?;
    let expected_kind = match inner.kind {
        InnerKind::ReceiptDelivered | InnerKind::ReceiptUndecryptable => EnvelopeKind::Receipt,
        _ => EnvelopeKind::Data,
    };
    if envelope.header.kind != expected_kind {
        return Err(CryptoError::Proto(vl_proto::ProtoError::InvalidField(
            "kind",
        )));
    }
    let vaccine = (envelope.header.vaccine_enabled()
        && ack_commitment(&inner.ack_secret) == envelope.header.ack_commitment)
        .then_some(inner.ack_secret);
    Ok(Opened { inner, vaccine })
}

/// Verifies a receipt from `pair`'s contact. Only this function's success may mark a message Delivered (ADR-015).
pub fn verify_receipt(
    pair: &PairKeys,
    envelope: &Envelope,
) -> Result<(InnerKind, Receipt), CryptoError> {
    let opened = open_from(pair, envelope)?;
    match opened.inner.kind {
        InnerKind::ReceiptDelivered | InnerKind::ReceiptUndecryptable => {
            Ok((opened.inner.kind, Receipt::decode(&opened.inner.payload)?))
        }
        _ => Err(CryptoError::Proto(vl_proto::ProtoError::InvalidField(
            "not a receipt",
        ))),
    }
}

/// Recipient-side: decodes the Olm payload of an opened TEXT or CONTENT inner message.
pub fn olm_payload(opened: &Opened) -> Result<OlmPayload, CryptoError> {
    if !matches!(opened.inner.kind, InnerKind::Text | InnerKind::Content) {
        return Err(CryptoError::Proto(vl_proto::ProtoError::InvalidField(
            "not text",
        )));
    }
    OlmPayload::decode(&opened.inner.payload)
}

#[cfg(test)]
mod tests {

    #[test]
    fn chunks_open_only_with_their_key_id_and_index() {
        let key = [7u8; 32];
        let sealed = seal_chunk(&key, &[1; 16], 3, b"part three").unwrap();
        assert_eq!(sealed.len(), 10 + 16);
        assert_eq!(
            open_chunk(&key, &[1; 16], 3, &sealed).unwrap(),
            b"part three"
        );
        assert!(
            open_chunk(&[8; 32], &[1; 16], 3, &sealed).is_err(),
            "wrong key"
        );
        assert!(
            open_chunk(&key, &[2; 16], 3, &sealed).is_err(),
            "moved to another attachment"
        );
        assert!(
            open_chunk(&key, &[1; 16], 4, &sealed).is_err(),
            "moved to another index"
        );
        let mut tampered = sealed.clone();
        tampered[0] ^= 1;
        assert!(open_chunk(&key, &[1; 16], 3, &tampered).is_err());
    }

    #[test]
    fn largest_sealed_chunk_fits_the_2048_bucket() {
        let chunk = ChunkPayload {
            attachment_id: [1; 16],
            index: 0,
            ciphertext: vec![0; vl_proto::CHUNK_BYTES + vl_proto::CHUNK_TAG_BYTES],
        };
        let needed = INNER_HEADER_LEN + chunk.encode().unwrap().len() + AEAD_OVERHEAD;
        assert_eq!(bucket_for(needed), Some(2048));
    }
    use super::*;
    use crate::{Identity, decrypt_olm};

    struct Party {
        id: Identity,
        card: vl_proto::PairingCard,
    }

    fn party(name: &str) -> Party {
        let mut id = Identity::generate().unwrap();
        let card = id.create_card(name, 3).unwrap();
        Party { id, card }
    }

    const P: SendParams = SendParams {
        now_secs: 1_790_000_000,
        sent_at_ms: 1_790_000_000_000,
        lifetime_min: 72 * 60,
        copy_budget: 4,
    };

    #[test]
    fn alice_to_charlie_and_receipt_back() {
        let (alice, mut charlie) = (party("Alice"), party("Charlie"));
        let a_keys = alice.id.pair(&alice.card, &charlie.card).unwrap();
        let c_keys = charlie.id.pair(&charlie.card, &alice.card).unwrap();
        let mut a_session = alice.id.outbound_session(&charlie.card, 0).unwrap();

        let out = build_text(&a_keys, &mut a_session, "meet at the north gate", P).unwrap();
        let wire = out.envelope.encode();

        let mut relayed = Envelope::decode(&wire).unwrap();
        relayed.on_relay_accept();
        relayed.header.copy_budget = 1;

        assert_eq!(
            relayed.header.recipient_tag,
            c_keys.tag_in(epoch_of(P.now_secs))
        );
        let opened = open_from(&c_keys, &relayed).unwrap();
        assert_eq!(opened.inner.message_id, out.message_id);
        assert_eq!(opened.vaccine, Some(out.ack_secret));
        let payload = olm_payload(&opened).unwrap();
        let (_session, plain) = decrypt_olm(
            &mut charlie.id,
            &alice.card.olm_identity_key,
            None,
            &payload,
        )
        .unwrap();
        assert_eq!(plain, b"meet at the north gate");

        let receipt =
            build_receipt(&c_keys, InnerKind::ReceiptDelivered, &[out.message_id], P).unwrap();
        let (kind, r) = verify_receipt(&a_keys, &receipt).unwrap();
        assert_eq!(kind, InnerKind::ReceiptDelivered);
        assert_eq!(r.refs, vec![out.message_id]);
    }

    #[test]
    fn forged_receipts_and_misdirected_envelopes_fail() {
        let (alice, charlie, mallory) = (party("A"), party("C"), party("M"));
        let a_keys = alice.id.pair(&alice.card, &charlie.card).unwrap();
        let m_keys_with_a = mallory.id.pair(&mallory.card, &alice.card).unwrap();
        let forged =
            build_receipt(&m_keys_with_a, InnerKind::ReceiptDelivered, &[[1; 16]], P).unwrap();
        assert_eq!(
            verify_receipt(&a_keys, &forged).err(),
            Some(CryptoError::Aead)
        );

        let c_keys = charlie.id.pair(&charlie.card, &alice.card).unwrap();
        let mut a_session = alice.id.outbound_session(&charlie.card, 0).unwrap();
        let text = build_text(&a_keys, &mut a_session, "hi", P).unwrap();
        assert!(
            verify_receipt(&a_keys, &text.envelope).is_err(),
            "own outgoing text is not a receipt"
        );
        assert!(
            open_from(&m_keys_with_a, &text.envelope).is_err(),
            "third party cannot open"
        );
        let mut retagged = text.envelope.clone();
        retagged.header.expiry_min += 1;
        assert_eq!(open_from(&c_keys, &retagged).err(), Some(CryptoError::Aead));
    }

    #[test]
    fn vaccine_only_released_when_commitment_matches() {
        let (alice, charlie) = (party("A"), party("C"));
        let a_keys = alice.id.pair(&alice.card, &charlie.card).unwrap();
        let c_keys = charlie.id.pair(&charlie.card, &alice.card).unwrap();
        let receipt = build_receipt(&a_keys, InnerKind::ReceiptDelivered, &[[2; 16]], P).unwrap();
        assert_eq!(open_from(&c_keys, &receipt).unwrap().vaccine, None);
    }

    #[test]
    fn largest_text_that_fits_one_envelope() {
        let (alice, charlie) = (party("A"), party("C"));
        let a_keys = alice.id.pair(&alice.card, &charlie.card).unwrap();
        let mut session = alice.id.outbound_session(&charlie.card, 0).unwrap();
        let mut max_fit = 0;
        for len in (1500..=2000).step_by(4) {
            let text = "x".repeat(len);
            if build_text(&a_keys, &mut session, &text, P).is_ok() {
                max_fit = len;
            }
        }
        assert!(
            max_fit >= 1700,
            "pre-key message must carry the 1,700-byte MVP limit; got {max_fit}"
        );
        println!("largest pre-key text that fits the 2048 bucket: {max_fit} bytes (step 4)");
    }

    // AC-W.1: the 1,700-byte limit still fits as a pre-key CONTENT message with
    // every optional field present.
    #[test]
    fn largest_content_text_fits_as_pre_key_message() {
        let (alice, mut charlie) = (party("A"), party("C"));
        let a_keys = alice.id.pair(&alice.card, &charlie.card).unwrap();
        let c_keys = charlie.id.pair(&charlie.card, &alice.card).unwrap();
        let mut session = alice.id.outbound_session(&charlie.card, 0).unwrap();
        let content = Content::Text {
            reply_to: Some([9; 16]),
            forwarded: true,
            timer_secs: vl_proto::MAX_TIMER_SECS,
            text: "é".repeat(850),
        };
        let out = build_content(&a_keys, &mut session, &content, P).unwrap();
        assert_eq!(out.envelope.header.body_len, 2048);
        let opened = open_from(&c_keys, &out.envelope).unwrap();
        assert_eq!(opened.inner.kind, InnerKind::Content);
        let payload = olm_payload(&opened).unwrap();
        assert!(payload.is_pre_key());
        let (_s, plain) = decrypt_olm(
            &mut charlie.id,
            &alice.card.olm_identity_key,
            None,
            &payload,
        )
        .unwrap();
        assert_eq!(Content::decode(&plain).unwrap(), content);
    }

    #[test]
    fn largest_group_text_fits_as_pre_key_message() {
        let (alice, charlie) = (party("A"), party("C"));
        let a_keys = alice.id.pair(&alice.card, &charlie.card).unwrap();
        let mut session = alice.id.outbound_session(&charlie.card, 0).unwrap();
        let content = Content::GroupText {
            group_id: [1; 16],
            group_msg_id: [2; 16],
            epoch: u32::MAX,
            reply_to: Some([3; 16]),
            text: "é".repeat(850),
        };
        let out = build_content(&a_keys, &mut session, &content, P).unwrap();
        assert_eq!(out.envelope.header.body_len, 2048);
    }

    #[test]
    fn expiry_is_jittered_within_bounds() {
        let now_min = u32::try_from(P.now_secs / 60).unwrap();
        for _ in 0..200 {
            let e = expiry(P.now_secs, P.lifetime_min).unwrap();
            assert!(
                e >= now_min + P.lifetime_min - EXPIRY_JITTER_MIN
                    && e <= now_min + P.lifetime_min + EXPIRY_JITTER_MIN
            );
        }
        for _ in 0..200 {
            let e = expiry(P.now_secs, 0).unwrap();
            assert!(
                e > now_min && e <= now_min + EXPIRY_JITTER_MIN,
                "never at or before now"
            );
        }
    }
}
