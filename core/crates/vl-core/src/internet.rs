//! Internet mailbox (ADR-039). Optional: when the user turns it on and the
//! phone or browser has a connection, the host uploads the same sealed
//! envelopes this node carries over Bluetooth and fetches those addressed to
//! its current tags. Nothing new is encrypted here and no new keys exist; the
//! mailbox is just one more relay that cannot read what it holds.

use crate::{CoreError, CoreResult, Node, now_min};
use vl_proto::Envelope;
use vl_routing::Event;

/// Envelopes per upload request (matches the mailbox's limit).
pub const INTERNET_BATCH: u32 = 64;

impl Node {
    /// Sealed envelopes the mailbox does not have yet: mine first, then mail
    /// carried for other people, so it reaches them if they are online.
    pub fn internet_outbox(&self, now_secs: u64) -> CoreResult<Vec<Vec<u8>>> {
        Ok(self.relay.not_uploaded(now_min(now_secs), INTERNET_BATCH)?)
    }

    /// Call after the mailbox accepted `envelopes`. My own messages become
    /// "Accepted by relay", exactly as when a nearby phone takes them.
    pub fn internet_uploaded(&mut self, envelopes: &[Vec<u8>], now_secs: u64) -> CoreResult<()> {
        let ids: Vec<[u8; 16]> = envelopes
            .iter()
            .filter_map(|w| Envelope::decode(w).ok())
            .map(|e| e.header.envelope_id)
            .collect();
        let own = self.relay.mark_uploaded(&ids)?;
        self.apply_events(own.into_iter().map(Event::OwnAccepted).collect(), now_secs)?;
        Ok(())
    }

    /// The tags to ask the mailbox for. They rotate every epoch, and the
    /// mailbox cannot tell whose they are.
    pub fn internet_tags(&self) -> CoreResult<Vec<[u8; 16]>> {
        Ok(self.relay.inbound_tags()?)
    }

    /// One envelope fetched from the mailbox. Returns true if it was new mail
    /// or a new receipt. Anything not for this phone, forged, expired or seen
    /// before is ignored, so a dishonest mailbox can only withhold mail.
    pub fn receive_internet(&mut self, wire: &[u8], now_secs: u64) -> CoreResult<bool> {
        let Ok(env) = Envelope::decode(wire) else {
            return Ok(false);
        };
        match self.receive_sealed(env, now_secs) {
            Ok((_, new)) => Ok(new),
            Err(CoreError::NotForThisPhone) => Ok(false),
            Err(e) => Err(e),
        }
    }
}
