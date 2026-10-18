//! QR "paper" messages (ADR-036). The QR carries exactly the sealed envelope
//! that Bluetooth would carry, so nothing about the encryption changes; the
//! recipient feeds it through the same path as a Bluetooth delivery.

use crate::{CoreError, CoreResult, Node, now_min};
use vl_proto::{Admission, Envelope, decode_paper_message, encode_paper_message};
use vl_routing::{Event, Store};

/// Largest envelope body shown as a QR: bigger codes are hard to scan.
pub const MAX_PAPER_BODY: u16 = 1024;

impl Node {
    /// The QR text for one of my undelivered messages. Fails if the envelope is
    /// gone (delivered and purged, or expired) or too large to scan reliably.
    pub fn paper_message(&self, message_id: &[u8; 16]) -> CoreResult<String> {
        let m = self
            .conv
            .message(message_id)?
            .filter(|m| m.outgoing)
            .ok_or(CoreError::UnknownMessage)?;
        let eid = m.envelope_id.ok_or(CoreError::PaperUnavailable)?;
        let stored = self.relay.get(&eid)?.ok_or(CoreError::PaperUnavailable)?;
        if stored.envelope.header.body_len > MAX_PAPER_BODY {
            return Err(CoreError::PaperUnavailable);
        }
        Ok(encode_paper_message(&stored.envelope.encode()))
    }

    /// Takes a scanned message code. Returns the contact it came from; the
    /// usual receipt goes back over Bluetooth at the next meeting.
    pub fn receive_paper(&mut self, code: &str, now_secs: u64) -> CoreResult<[u8; 16]> {
        let env = Envelope::decode(&decode_paper_message(code.trim())?)?;
        Ok(self.receive_sealed(env, now_secs)?.0)
    }

    /// One sealed envelope that arrived outside a Bluetooth link (QR code or
    /// internet mailbox), through the same checks as a Bluetooth delivery.
    /// Returns the sending contact and whether it was new.
    pub(crate) fn receive_sealed(
        &mut self,
        env: Envelope,
        now_secs: u64,
    ) -> CoreResult<([u8; 16], bool)> {
        let h = &env.header;
        match h.admission(
            now_min(now_secs),
            self.policy.max_lifetime_min,
            self.policy.skew_min,
        ) {
            Admission::Expired | Admission::LifetimeTooLong => {
                return Err(CoreError::NotForThisPhone);
            }
            _ => {}
        }
        let contact_ref = self
            .relay
            .contact_for_tag(&h.recipient_tag)?
            .ok_or(CoreError::NotForThisPhone)?;
        let contact = self.contact_by_ref(&contact_ref)?;
        let id = h.envelope_id;
        if self.relay.seen(&id)? {
            return Ok((contact.contact_id, false));
        }
        self.relay.mark_seen(&id, h.expiry_min)?;
        // A forged or corrupted envelope fails authentication here, and its id
        // is released again (see on_inbound).
        self.apply_events(vec![Event::Inbound(env)], now_secs)?;
        if !self.relay.seen(&id)? {
            return Err(CoreError::NotForThisPhone);
        }
        Ok((contact.contact_id, true))
    }
}
