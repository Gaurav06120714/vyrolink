use crate::{CoreError, CoreResult, Node, now_min, secs_i64};
use std::collections::HashSet;
use vl_crypto::{LinkHandshake, LinkTransport, random_array};
use vl_proto::{
    Deframer, FEATURE_PRESENCE, LinkMessage, MAX_FRAME_LEN, MAX_ITEMS, PRESENCE_PAD, frame,
};
use vl_routing::LinkSession;

/// A contact seen again after at least this long away counts as a new arrival.
pub const ARRIVAL_GAP_SECS: u64 = 30 * 60;

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
    /// Noise handshake hash; presence tags are bound to it (ADR-029).
    handshake_hash: [u8; 32],
    presence_sent: bool,
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
                handshake_hash: [0; 32],
                presence_sent: false,
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
                    link.handshake_hash = hs.handshake_hash()?;
                    let mut t = (*hs).into_transport()?;
                    for mut m in link.session.open(&self.relay)? {
                        if let LinkMessage::Hello(h) = &mut m {
                            h.features |= FEATURE_PRESENCE;
                        }
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
                let mut send = Vec::new();
                match &msg {
                    LinkMessage::Presence(tags) => {
                        self.on_presence(tags, &link.handshake_hash, now_secs)?;
                        link.phase = Phase::Transport(t);
                        return Ok(());
                    }
                    LinkMessage::Hello(h)
                        if h.features & FEATURE_PRESENCE != 0 && !link.presence_sent =>
                    {
                        link.presence_sent = true;
                        send.push(LinkMessage::Presence(
                            self.presence_tags(&link.handshake_hash)?,
                        ));
                    }
                    _ => {}
                }
                let (replies, events) = link.session.handle(msg, &mut self.relay, now);
                send.extend(replies);
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

impl Node {
    /// This device's presence tags for one link: one per unblocked contact,
    /// padded with random values to a multiple of [`PRESENCE_PAD`] so the peer
    /// learns only a coarse contact count (ADR-029).
    fn presence_tags(&self, handshake_hash: &[u8; 32]) -> CoreResult<Vec<[u8; 16]>> {
        let mut tags = Vec::new();
        // ponytail: only the first 128 contacts (MAX_ITEMS) are announced; rotate
        // which ones per link if anyone ever has more.
        for c in self
            .conv
            .contacts()?
            .into_iter()
            .filter(|c| !c.blocked)
            .take(MAX_ITEMS)
        {
            tags.push(self.pair_keys(&c.contact_ref)?.presence_out(handshake_hash));
        }
        let padded = tags.len().div_ceil(PRESENCE_PAD).max(1) * PRESENCE_PAD;
        while tags.len() < padded.min(MAX_ITEMS) {
            tags.push(random_array()?);
        }
        Ok(tags)
    }

    /// Records every unblocked contact whose tag the peer showed on this link.
    fn on_presence(
        &mut self,
        tags: &[[u8; 16]],
        handshake_hash: &[u8; 32],
        now_secs: u64,
    ) -> CoreResult<()> {
        let shown: HashSet<&[u8; 16]> = tags.iter().collect();
        for c in self.conv.contacts()?.into_iter().filter(|c| !c.blocked) {
            if shown.contains(&self.pair_keys(&c.contact_ref)?.presence_in(handshake_hash)) {
                let prev = self.nearby.insert(c.contact_id, now_secs);
                // ADR-033: keep only the latest sighting, encrypted at rest.
                self.conv.touch_seen(&c.contact_id, secs_i64(now_secs))?;
                if prev.is_none_or(|p| now_secs.saturating_sub(p) >= ARRIVAL_GAP_SECS) {
                    self.nearby_arrivals = self.nearby_arrivals.saturating_add(1);
                }
            }
        }
        Ok(())
    }

    /// How many contacts came into range (first time, or after
    /// [`ARRIVAL_GAP_SECS`] away) since the last call. Hosts use it for one
    /// generic "A contact is nearby" notification; it never says who.
    pub fn take_nearby_arrivals(&mut self) -> u32 {
        std::mem::take(&mut self.nearby_arrivals)
    }

    /// Unblocked contacts seen over Bluetooth within `within_secs`, with the
    /// time they were last seen. Nothing here survives a restart.
    pub fn nearby_contacts(
        &self,
        now_secs: u64,
        within_secs: u64,
    ) -> CoreResult<Vec<([u8; 16], u64)>> {
        let blocked: HashSet<[u8; 16]> = self
            .conv
            .contacts()?
            .into_iter()
            .filter(|c| c.blocked)
            .map(|c| c.contact_id)
            .collect();
        Ok(self
            .nearby
            .iter()
            .filter(|(id, seen)| {
                !blocked.contains(*id) && now_secs.saturating_sub(**seen) <= within_secs
            })
            .map(|(id, seen)| (*id, *seen))
            .collect())
    }
}
