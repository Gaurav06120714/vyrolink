use crate::ProtoError;

/// Prefixes `payload` with its u16 big-endian length.
pub fn frame(payload: &[u8]) -> Result<Vec<u8>, ProtoError> {
    let len = u16::try_from(payload.len()).map_err(|_| ProtoError::BadLength)?;
    if len == 0 {
        return Err(ProtoError::BadLength);
    }
    let mut out = Vec::with_capacity(payload.len() + 2);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(payload);
    Ok(out)
}

/// Reassembles length-prefixed frames from an ordered byte stream delivered in
/// arbitrary chunks (GATT writes/notifications or an L2CAP stream).
///
/// Memory is bounded: at most one incomplete frame (≤ `max_frame` bytes plus
/// its 2-byte prefix) is buffered. A zero-length or oversized frame poisons the
/// stream, because resynchronising an untrusted stream is not possible; the
/// caller must drop the link.
#[derive(Debug)]
pub struct Deframer {
    buf: Vec<u8>,
    max_frame: usize,
    poisoned: bool,
}

impl Deframer {
    pub fn new(max_frame: usize) -> Self {
        Self {
            buf: Vec::new(),
            max_frame,
            poisoned: false,
        }
    }

    /// Appends `chunk` and returns every frame it completes.
    pub fn push(&mut self, mut chunk: &[u8]) -> Result<Vec<Vec<u8>>, ProtoError> {
        if self.poisoned {
            return Err(ProtoError::BadLength);
        }
        let mut frames = Vec::new();
        while !chunk.is_empty() {
            let need = match self.declared_len() {
                None => 2 - self.buf.len(),
                Some(len) => 2 + len - self.buf.len(),
            };
            let n = need.min(chunk.len());
            let (head, tail) = chunk.split_at(n);
            self.buf.extend_from_slice(head);
            chunk = tail;
            if let Some(len) = self.declared_len() {
                if len == 0 || len > self.max_frame {
                    self.poisoned = true;
                    self.buf = Vec::new();
                    return Err(ProtoError::BadLength);
                }
                if self.buf.len() == 2 + len {
                    let payload = self.buf.split_off(2);
                    self.buf.clear();
                    frames.push(payload);
                }
            }
        }
        Ok(frames)
    }

    fn declared_len(&self) -> Option<usize> {
        match self.buf.as_slice() {
            [a, b, ..] => Some(usize::from(u16::from_be_bytes([*a, *b]))),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reassembles_across_arbitrary_chunk_boundaries() {
        let a = frame(b"hello").unwrap();
        let b = frame(&[7u8; 300]).unwrap();
        let stream: Vec<u8> = a.iter().chain(b.iter()).copied().collect();
        for chunk_size in 1..=stream.len() {
            let mut d = Deframer::new(1024);
            let mut out = Vec::new();
            for chunk in stream.chunks(chunk_size) {
                out.extend(d.push(chunk).unwrap());
            }
            assert_eq!(out, vec![b"hello".to_vec(), vec![7u8; 300]]);
        }
    }

    #[test]
    fn oversized_and_empty_frames_poison_the_stream() {
        let mut d = Deframer::new(10);
        assert_eq!(d.push(&[0, 11]), Err(ProtoError::BadLength));
        assert_eq!(d.push(&[0, 1, 9]), Err(ProtoError::BadLength));
        let mut z = Deframer::new(10);
        assert_eq!(z.push(&[0, 0]), Err(ProtoError::BadLength));
        assert_eq!(frame(&[]), Err(ProtoError::BadLength));
    }
}
