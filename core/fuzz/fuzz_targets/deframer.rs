#![no_main]
use libfuzzer_sys::fuzz_target;
use vl_proto::{Deframer, MAX_FRAME_LEN};

fuzz_target!(|data: &[u8]| {
    let split = data.first().map(|b| usize::from(*b).max(1)).unwrap_or(1);
    let mut d = Deframer::new(MAX_FRAME_LEN);
    for chunk in data.chunks(split) {
        match d.push(chunk) {
            Ok(frames) => assert!(frames.iter().all(|f| !f.is_empty() && f.len() <= MAX_FRAME_LEN)),
            Err(_) => break,
        }
    }
});
