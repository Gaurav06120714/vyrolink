#![no_main]
use libfuzzer_sys::fuzz_target;
use vl_proto::*;

fuzz_target!(|data: &[u8]| {
    if let Ok(h) = Header::decode(data) {
        assert_eq!(Header::decode(&h.encode()), Ok(h));
    }
    if let Ok(e) = Envelope::decode(data) {
        assert_eq!(e.encode(), data);
    }
    if let Ok(m) = LinkMessage::decode(data) {
        let again = m.encode().expect("decoded message must re-encode");
        assert_eq!(LinkMessage::decode(&again), Ok(m));
    }
    if let Ok(i) = Inner::decode(data) {
        assert_eq!(Inner::decode(&i.encode_padded(data.len()).expect("fits")), Ok(i));
    }
    if let Ok(r) = Receipt::decode(data) {
        assert_eq!(r.encode().expect("re-encode"), data);
    }
    if let Ok(c) = Content::decode(data) {
        assert_eq!(c.encode().expect("re-encode"), data);
    }
    if let Ok(c) = PairingCard::decode(data) {
        assert_eq!(c.encode().expect("re-encode"), data);
    }
    if let Ok(c) = ChunkPayload::decode(data) {
        assert_eq!(c.encode().expect("re-encode"), data);
    }
    if let Ok(text) = std::str::from_utf8(data) {
        if let Ok(card) = decode_pairing_code(text) {
            assert_eq!(encode_pairing_code(&card), text);
        }
    }
});
