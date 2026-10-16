use crate::{CoreError, CoreResult, Node, now_min};
use vl_crypto::{LinkHandshake, LinkTransport};
use vl_proto::{Deframer, LinkMessage, MAX_FRAME_LEN, frame};
use vl_routing::LinkSession;

/// Noise adds a 16-byte tag to every transport message.
const MAX_WIRE_FRAME: usize = MAX_FRAME_LEN + 16;

pub type LinkId = u64;

enum Phase {
    Handshake(Box<LinkHandshake>),
    Transport(LinkTransport),
    Failed,
}

/// One encrypted neighbour connection: framing → Noise NN → link messages → routing session.
pub(crate) struct Link {
    phase: Phase,
    initiator: bool,
    deframer: Deframer,
    session: LinkSession,
}

impl Node {
    /// Starts a link over a freshly connected byte stream. Returns the bytes to send first.
    pub fn link_open(&mut self, initiator: bool) -> CoreResult<(LinkId, Vec<u8>)> {
        let mut hs = if initiator {
            LinkHandshake::initiator()?
        } else {
            LinkHandshake::responder()?
        };
        let first = if initiator {
            frame(&hs.write_message()?)?
        } else {
            Vec::new()
        };
        let id = self.next_link;
        self.next_link += 1;
        self.links.insert(
            id,
            Link {
                phase: Phase::Handshake(Box::new(hs)),
                initiator,
                deframer: Deframer::new(MAX_WIRE_FRAME),
                session: LinkSession::new(self.policy),
            },
        );
        Ok((id, first))
    }

    /// Feeds bytes received on a link; returns bytes to send back. Any protocol or
    /// crypto failure closes the link (the transport should then disconnect).
    pub fn link_receive(&mut self, id: LinkId, bytes: &[u8], now_secs: u64) -> CoreResult<Vec<u8>> {
        let mut link = self.links.remove(&id).ok_or(CoreError::UnknownLink)?;
        let result = self.drive(&mut link, bytes, now_secs);
        if result.is_ok() && !matches!(link.phase, Phase::Failed) && !link.session.is_closed() {
            self.links.insert(id, link);
        }
        result
    }

    pub fn link_close(&mut self, id: LinkId) {
        self.links.remove(&id);
    }

    pub fn link_is_open(&self, id: LinkId) -> bool {
        self.links.contains_key(&id)
    }

    fn drive(&mut self, link: &mut Link, bytes: &[u8], now_secs: u64) -> CoreResult<Vec<u8>> {
        let mut out = Vec::new();
        let frames = match link.deframer.push(bytes) {
            Ok(f) => f,
            Err(e) => {
                link.phase = Phase::Failed;
                return Err(e.into());
            }
        };
        for f in frames {
            if let Err(e) = self.on_frame(link, &f, now_secs, &mut out) {
                link.phase = Phase::Failed;
                return Err(e);
            }
        }
        Ok(out)
    }

    fn on_frame(
        &mut self,
        link: &mut Link,
        f: &[u8],
        now_secs: u64,
        out: &mut Vec<u8>,
    ) -> CoreResult<()> {
        match std::mem::replace(&mut link.phase, Phase::Failed) {
            Phase::Handshake(mut hs) => {
                hs.read_message(f)?;
                if !link.initiator {
                    out.extend(frame(&hs.write_message()?)?);
                }
                if hs.is_finished() {
                    let mut t = (*hs).into_transport()?;
                    for m in link.session.open(&self.relay)? {
                        out.extend(frame(&t.encrypt(&m.encode()?)?)?);
                    }
                    link.phase = Phase::Transport(t);
                } else {
                    link.phase = Phase::Handshake(hs);
                }
            }
            Phase::Transport(mut t) => {
                let msg = LinkMessage::decode(&t.decrypt(f)?)?;
                let now = now_min(now_secs);
                let (replies, events) = link.session.handle(msg, &mut self.relay, now);
                let mut send = replies;
                if self.apply_events(events, now_secs)? && !link.session.is_closed() {
                    send.extend(link.session.refresh(&self.relay, now)?);
                }
                for m in send {
                    out.extend(frame(&t.encrypt(&m.encode()?)?)?);
                }
                link.phase = Phase::Transport(t);
            }
            Phase::Failed => return Err(CoreError::UnknownLink),
        }
        Ok(())
    }
}
