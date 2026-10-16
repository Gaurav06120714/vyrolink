#![allow(clippy::unwrap_used, clippy::indexing_slicing)]
use proptest::prelude::*;
use vl_proto::*;

fn header_strategy() -> impl Strategy<Value = Header> {
    (
        prop_oneof![Just(EnvelopeKind::Data), Just(EnvelopeKind::Receipt)],
        prop_oneof![Just(0u8), Just(FLAG_VACCINE)],
        any::<u8>(),
        any::<u8>(),
        any::<u8>(),
        any::<[u8; 16]>(),
        any::<[u8; 16]>(),
        any::<u32>(),
        any::<[u8; 16]>(),
        prop::sample::select(BODY_BUCKETS.to_vec()),
    )
        .prop_map(
            |(
                kind,
                flags,
                hop_count,
                hop_limit,
                copy_budget,
                envelope_id,
                recipient_tag,
                expiry_min,
                ack_commitment,
                body_len,
            )| {
                Header {
                    kind,
                    flags,
                    hop_count,
                    hop_limit,
                    copy_budget,
                    envelope_id,
                    recipient_tag,
                    expiry_min,
                    ack_commitment,
                    body_len,
                }
            },
        )
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 4096, failure_persistence: Some(Box::new(proptest::test_runner::FileFailurePersistence::WithSource("regressions"))), ..ProptestConfig::default() })]

    #[test]
    fn decoders_never_panic_on_arbitrary_input(bytes in prop::collection::vec(any::<u8>(), 0..2400)) {
        let _ = Header::decode(&bytes);
        let _ = Envelope::decode(&bytes);
        let _ = LinkMessage::decode(&bytes);
        let _ = Inner::decode(&bytes);
        let _ = Receipt::decode(&bytes);
        let _ = PairingCard::decode(&bytes);
        if let Ok(c) = Content::decode(&bytes) {
            prop_assert_eq!(Content::decode(&c.encode().unwrap()).unwrap(), c);
        }
        let mut d = Deframer::new(MAX_FRAME_LEN);
        let _ = d.push(&bytes);
    }

    #[test]
    fn header_round_trip(h in header_strategy()) {
        prop_assert_eq!(Header::decode(&h.encode()), Ok(h));
    }

    #[test]
    fn envelope_round_trip_and_single_bit_flip_never_panics(
        h in header_strategy(),
        flip in any::<prop::sample::Index>(),
    ) {
        let env = Envelope { body: vec![0xA5; usize::from(h.body_len)], header: h };
        let mut bytes = env.encode();
        prop_assert_eq!(Envelope::decode(&bytes), Ok(env));
        let i = flip.index(bytes.len());
        bytes[i] ^= 0x01;
        let _ = Envelope::decode(&bytes);
    }

    #[test]
    fn link_summary_round_trip(
        ids in prop::collection::vec((any::<[u8; 16]>(), any::<[u8; 16]>(), any::<u32>(), prop::sample::select(BODY_BUCKETS.to_vec()), any::<bool>()), 0..=MAX_ITEMS),
        delivered in prop::collection::vec(any::<[u8; 16]>(), 0..=MAX_ITEMS),
    ) {
        let entries = ids
            .into_iter()
            .map(|(envelope_id, recipient_tag, expiry_min, body_len, spray)| SummaryEntry { envelope_id, recipient_tag, expiry_min, body_len, spray })
            .collect();
        let msg = LinkMessage::Summary { entries, delivered };
        let bytes = msg.encode().unwrap();
        prop_assert!(bytes.len() <= MAX_FRAME_LEN);
        prop_assert_eq!(LinkMessage::decode(&bytes), Ok(msg));
    }

    #[test]
    fn inner_round_trip(payload in prop::collection::vec(any::<u8>(), 0..1500), extra in 0usize..400) {
        let inner = Inner { kind: InnerKind::Text, message_id: [7; 16], ack_secret: [8; 16], sent_at_ms: 42, payload };
        let len = INNER_HEADER_LEN + inner.payload.len() + extra;
        prop_assert_eq!(Inner::decode(&inner.encode_padded(len).unwrap()), Ok(inner));
    }

    #[test]
    fn deframer_reassembles_any_chunking(
        payloads in prop::collection::vec(prop::collection::vec(any::<u8>(), 1..300), 1..8),
        chunk in 1usize..64,
    ) {
        let stream: Vec<u8> = payloads.iter().flat_map(|p| frame(p).unwrap()).collect();
        let mut d = Deframer::new(300);
        let mut out = Vec::new();
        for c in stream.chunks(chunk) {
            out.extend(d.push(c).unwrap());
        }
        prop_assert_eq!(out, payloads);
    }
}
