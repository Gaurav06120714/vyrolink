//! Introductions (ADR-037): a contact you trust vouches for someone you have
//! not met. Weaker than meeting in person, so introduced contacts start
//! unverified. Each side that accepts makes a fresh signed card, so no
//! one-time key is ever reused; each card must be signed by the identity the
//! introducer named.

use crate::{CoreError, CoreResult, Node, ONE_TIME_KEYS_PER_CARD, secs_i64};
use vl_crypto::{random_array, verify_card};
use vl_proto::{Content, PairingCard};
use vl_store::{Contact, IntroRow, MessageRow, MessageState};

/// Introduction states (`IntroRow::state`).
pub const INTRO_ASKED: u8 = 0;
pub const INTRO_ACCEPTED: u8 = 1;
pub const INTRO_DECLINED: u8 = 2;
pub const INTRO_CONNECTED: u8 = 3;
pub const INTRO_FAILED: u8 = 4;

impl Node {
    /// Introduces two of my contacts to each other: each is asked first.
    pub fn introduce(&mut self, a: &[u8; 16], b: &[u8; 16], now_secs: u64) -> CoreResult<[u8; 16]> {
        if a == b {
            return Err(CoreError::NotAllowed);
        }
        let (ca, cb) = (self.contact(a)?, self.contact(b)?);
        if ca.blocked || cb.blocked {
            return Err(CoreError::ContactBlocked);
        }
        let intro_id: [u8; 16] = random_array()?;
        for (to, other) in [(&ca, &cb), (&cb, &ca)] {
            let content = Content::IntroAsk {
                intro_id,
                other_fp: Self::fingerprint_of(other)?,
                other_name: other.display_name.clone(),
            };
            let out = self.send_content(to, &content, now_secs)?;
            let header = &out.envelope.header;
            self.conv.insert_message(&MessageRow {
                message_id: out.message_id,
                contact_id: to.contact_id,
                outgoing: true,
                body: format!("👋 Introduced you to {}", other.display_name),
                created_at: secs_i64(now_secs),
                state: MessageState::Queued,
                envelope_id: Some(header.envelope_id),
                ack_secret: Some(out.ack_secret),
                expiry_min: Some(header.expiry_min),
                sent_at_ms: i64::try_from(out.sent_at_ms).unwrap_or(i64::MAX),
                ..Default::default()
            })?;
        }
        self.conv.put_intro(&IntroRow {
            intro_id,
            role: 0,
            a_contact: Some(*a),
            b_contact: Some(*b),
            state: INTRO_ASKED,
            created_at: secs_i64(now_secs),
            ..Default::default()
        })?;
        Ok(intro_id)
    }

    /// Accepts an introduction: sends the introducer a fresh card for them to pass on.
    pub fn accept_intro(
        &mut self,
        intro_id: &[u8; 16],
        my_name: &str,
        now_secs: u64,
    ) -> CoreResult<()> {
        let mut i = self
            .conv
            .intro(intro_id)?
            .filter(|i| i.role == 1)
            .ok_or(CoreError::UnknownMessage)?;
        if i.state != INTRO_ASKED {
            return Err(CoreError::NotAllowed);
        }
        let introducer = self.contact(&i.introducer.ok_or(CoreError::UnknownContact)?)?;
        let card = self
            .identity
            .as_mut()
            .ok_or(CoreError::NoIdentity)?
            .create_card(my_name, ONE_TIME_KEYS_PER_CARD)?;
        self.persist_identity(now_secs)?;
        let content = Content::IntroCard {
            intro_id: *intro_id,
            card: card.encode()?,
        };
        self.send_content(&introducer, &content, now_secs)?;
        i.state = INTRO_ACCEPTED;
        self.conv.put_intro(&i)?;
        Ok(())
    }

    pub fn decline_intro(&mut self, intro_id: &[u8; 16]) -> CoreResult<()> {
        let mut i = self
            .conv
            .intro(intro_id)?
            .filter(|i| i.role == 1)
            .ok_or(CoreError::UnknownMessage)?;
        if i.state == INTRO_ASKED {
            i.state = INTRO_DECLINED;
            self.conv.put_intro(&i)?;
        }
        Ok(())
    }

    pub fn intro_for_message(&self, message_id: &[u8; 16]) -> CoreResult<Option<IntroRow>> {
        Ok(self.conv.intro_by_message(message_id)?)
    }

    /// Who introduced me to this contact, if they came from an introduction.
    pub(crate) fn introduced_by(&self, fp: &[u8; 32]) -> CoreResult<Option<String>> {
        let Some(id) = self.conv.introducer_of(fp)? else {
            return Ok(None);
        };
        Ok(self.contact(&id).ok().map(|c| c.display_name))
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn on_intro_ask(
        &mut self,
        from: &Contact,
        message_id: [u8; 16],
        sent_at_ms: i64,
        intro_id: [u8; 16],
        other_fp: [u8; 32],
        other_name: String,
        now_secs: u64,
    ) -> CoreResult<()> {
        if other_fp == self.my_fingerprint()? || self.conv.intro(&intro_id)?.is_some() {
            return Ok(());
        }
        let known = self.contact_by_fingerprint(&other_fp)?.is_some();
        self.conv.insert_message(&MessageRow {
            message_id,
            contact_id: from.contact_id,
            outgoing: false,
            body: format!(
                "👋 {} wants to introduce you to {other_name}",
                from.display_name
            ),
            created_at: secs_i64(now_secs),
            state: MessageState::Received,
            sent_at_ms,
            ..Default::default()
        })?;
        self.conv.put_intro(&IntroRow {
            intro_id,
            role: 1,
            introducer: Some(from.contact_id),
            other_name,
            other_fp: Some(other_fp),
            state: if known { INTRO_CONNECTED } else { INTRO_ASKED },
            message_id: Some(message_id),
            created_at: secs_i64(now_secs),
            ..Default::default()
        })?;
        Ok(())
    }

    /// A fresh card from one side of my introduction. It must be signed by
    /// that contact's own identity. Once both arrive, each gets the other's.
    pub(crate) fn on_intro_card(
        &mut self,
        from: &Contact,
        intro_id: [u8; 16],
        card_bytes: Vec<u8>,
        now_secs: u64,
    ) -> CoreResult<()> {
        let Some(mut i) = self
            .conv
            .intro(&intro_id)?
            .filter(|i| i.role == 0 && i.state == INTRO_ASKED)
        else {
            return Ok(());
        };
        let card = PairingCard::decode(&card_bytes)?;
        verify_card(&card)?;
        if card.olm_fingerprint_key != Self::fingerprint_of(from)? {
            return Ok(());
        }
        if i.a_contact == Some(from.contact_id) {
            i.card_a = Some(card_bytes);
        } else if i.b_contact == Some(from.contact_id) {
            i.card_b = Some(card_bytes);
        } else {
            return Ok(());
        }
        if let (Some(ca), Some(cb), Some(a), Some(b)) =
            (&i.card_a, &i.card_b, i.a_contact, i.b_contact)
        {
            let (to_a, to_b) = (self.contact(&a)?, self.contact(&b)?);
            self.send_content(
                &to_a,
                &Content::IntroDeliver {
                    intro_id,
                    card: cb.clone(),
                },
                now_secs,
            )?;
            self.send_content(
                &to_b,
                &Content::IntroDeliver {
                    intro_id,
                    card: ca.clone(),
                },
                now_secs,
            )?;
            i.state = INTRO_CONNECTED;
        }
        self.conv.put_intro(&i)?;
        Ok(())
    }

    /// The other person's card from the introducer, for an introduction I
    /// accepted. It must be signed by the identity the introducer named.
    pub(crate) fn on_intro_deliver(
        &mut self,
        from: &Contact,
        intro_id: [u8; 16],
        card_bytes: Vec<u8>,
        now_secs: u64,
    ) -> CoreResult<()> {
        let Some(mut i) = self.conv.intro(&intro_id)?.filter(|i| {
            i.role == 1 && i.introducer == Some(from.contact_id) && i.state == INTRO_ACCEPTED
        }) else {
            return Ok(());
        };
        let card = PairingCard::decode(&card_bytes)?;
        if verify_card(&card).is_err() || Some(card.olm_fingerprint_key) != i.other_fp {
            i.state = INTRO_FAILED;
            self.conv.put_intro(&i)?;
            return Ok(());
        }
        if self
            .contact_by_fingerprint(&card.olm_fingerprint_key)?
            .is_none()
        {
            let keys = self
                .identity
                .as_ref()
                .ok_or(CoreError::NoIdentity)?
                .pair_with(&card)?;
            let contact = Contact {
                contact_id: random_array()?,
                contact_ref: random_array()?,
                display_name: card.display_name.clone(),
                card: card_bytes,
                verified: false,
                paired_at: secs_i64(now_secs),
                next_otk: 0,
                blocked: false,
            };
            self.relay
                .put_pair_keys(&contact.contact_ref, &keys.to_bytes())?;
            self.refresh_tags_for(&contact.contact_ref, &keys, now_secs)?;
            self.conv.add_contact(&contact)?;
        }
        i.state = INTRO_CONNECTED;
        self.conv.put_intro(&i)?;
        Ok(())
    }
}
