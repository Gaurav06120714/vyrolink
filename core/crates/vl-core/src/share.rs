//! Shared locations (ADR-034) and polls (ADR-035). Both travel as ordinary
//! Olm content events, so relays see nothing new; polls in groups fan out to
//! each member like group messages.

use crate::{CoreError, CoreResult, Node, secs_i64};
use vl_crypto::random_array;
use vl_proto::{Content, GroupRef};
use vl_store::{
    Contact, GroupMessageRow, LocationRow, MessageRow, MessageState, PendingGroupMessage, PollRow,
};

/// Text stored with a location message so lists and search show something.
pub const LOCATION_BODY: &str = "📍 Location";

/// A poll as shown to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PollView {
    pub question: String,
    pub options: Vec<String>,
    pub multi: bool,
    /// Votes per option.
    pub counts: Vec<u32>,
    /// This phone's current choice (bit i = option i).
    pub mine: u16,
    /// How many people have voted.
    pub voters: u32,
}

fn clean_options(options: &[String]) -> Vec<String> {
    options
        .iter()
        .map(|o| o.replace(['\n', '\r'], " ").trim().to_owned())
        .collect()
}

impl Node {
    /// Sends this phone's position to one contact (ADR-034). The host reads the
    /// GPS fix only when the user taps "Share location".
    pub fn send_location(
        &mut self,
        contact_id: &[u8; 16],
        lat_e7: i32,
        lon_e7: i32,
        accuracy_m: u16,
        captured_ms: u64,
        now_secs: u64,
    ) -> CoreResult<[u8; 16]> {
        let contact = self.contact(contact_id)?;
        let content = Content::Location {
            lat_e7,
            lon_e7,
            accuracy_m,
            captured_ms,
        };
        let out = self.send_content(&contact, &content, now_secs)?;
        let header = &out.envelope.header;
        self.conv.insert_message(&MessageRow {
            message_id: out.message_id,
            contact_id: *contact_id,
            outgoing: true,
            body: LOCATION_BODY.to_owned(),
            created_at: secs_i64(now_secs),
            state: MessageState::Queued,
            envelope_id: Some(header.envelope_id),
            ack_secret: Some(out.ack_secret),
            expiry_min: Some(header.expiry_min),
            sent_at_ms: i64::try_from(out.sent_at_ms).unwrap_or(i64::MAX),
            ..Default::default()
        })?;
        self.conv.put_location(
            &out.message_id,
            &LocationRow {
                lat_e7,
                lon_e7,
                accuracy_m,
                captured_ms,
            },
        )?;
        Ok(out.message_id)
    }

    pub fn location(&self, message_id: &[u8; 16]) -> CoreResult<Option<LocationRow>> {
        Ok(self.conv.location(message_id)?)
    }

    pub(crate) fn on_location(
        &mut self,
        contact: &Contact,
        message_id: [u8; 16],
        sent_at_ms: i64,
        row: LocationRow,
        now_secs: u64,
    ) -> CoreResult<()> {
        self.conv.insert_message(&MessageRow {
            message_id,
            contact_id: contact.contact_id,
            outgoing: false,
            body: LOCATION_BODY.to_owned(),
            created_at: secs_i64(now_secs),
            state: MessageState::Received,
            sent_at_ms,
            ..Default::default()
        })?;
        if self.conv.message(&message_id)?.is_some_and(|m| !m.deleted) {
            self.conv.put_location(&message_id, &row)?;
        }
        Ok(())
    }

    /// Starts a poll in a chat. The poll is shown under its own message id.
    pub fn send_poll(
        &mut self,
        contact_id: &[u8; 16],
        question: &str,
        options: &[String],
        multi: bool,
        now_secs: u64,
    ) -> CoreResult<[u8; 16]> {
        let contact = self.contact(contact_id)?;
        let poll_id: [u8; 16] = random_array()?;
        let options = clean_options(options);
        let question = question.trim().to_owned();
        let content = Content::Poll {
            group: None,
            poll_id,
            question: question.clone(),
            options: options.clone(),
            multi,
        };
        let out =
            self.send_content_opts(&contact, &content.encode()?, now_secs, None, Some(poll_id))?;
        let header = &out.envelope.header;
        self.conv.insert_message(&MessageRow {
            message_id: poll_id,
            contact_id: *contact_id,
            outgoing: true,
            body: question.clone(),
            created_at: secs_i64(now_secs),
            state: MessageState::Queued,
            envelope_id: Some(header.envelope_id),
            ack_secret: Some(out.ack_secret),
            expiry_min: Some(header.expiry_min),
            sent_at_ms: i64::try_from(out.sent_at_ms).unwrap_or(i64::MAX),
            ..Default::default()
        })?;
        self.conv.put_poll(
            &poll_id,
            &PollRow {
                question,
                options,
                multi,
            },
        )?;
        Ok(poll_id)
    }

    /// Starts a poll in a group: one pairwise copy per member.
    pub fn send_group_poll(
        &mut self,
        group_id: &[u8; 16],
        question: &str,
        options: &[String],
        multi: bool,
        now_secs: u64,
    ) -> CoreResult<[u8; 16]> {
        let g = self.conv.group(group_id)?.ok_or(CoreError::UnknownGroup)?;
        let me = self.my_fingerprint()?;
        if g.left || !g.members.iter().any(|(fp, _)| *fp == me) {
            return Err(CoreError::NotAllowed);
        }
        let poll_id: [u8; 16] = random_array()?;
        let options = clean_options(options);
        let question = question.trim().to_owned();
        let content = Content::Poll {
            group: Some(GroupRef {
                group_id: *group_id,
                epoch: g.epoch,
            }),
            poll_id,
            question: question.clone(),
            options: options.clone(),
            multi,
        };
        let encoded = content.encode()?;
        let mut sent_at_ms = 0;
        for (fp, _) in g.members.iter().filter(|(fp, _)| *fp != me) {
            let Some(c) = self.contact_by_fingerprint(fp)?.filter(|c| !c.blocked) else {
                continue;
            };
            let out = self.send_content_bytes(&c, &encoded, now_secs)?;
            sent_at_ms = i64::try_from(out.sent_at_ms).unwrap_or(i64::MAX);
            self.conv.add_group_copy(
                &out.message_id,
                &poll_id,
                &c.contact_id,
                &out.envelope.header.envelope_id,
                &out.ack_secret,
                out.envelope.header.expiry_min,
            )?;
        }
        self.conv.insert_group_message(&GroupMessageRow {
            group_msg_id: poll_id,
            group_id: *group_id,
            author_fp: me,
            outgoing: true,
            body: question.clone(),
            reply_to: None,
            created_at: secs_i64(now_secs),
            sent_at_ms,
        })?;
        self.conv.put_poll(
            &poll_id,
            &PollRow {
                question,
                options,
                multi,
            },
        )?;
        Ok(poll_id)
    }

    /// Sets this phone's whole choice for a poll (0 withdraws) and sends it to
    /// everyone who can see the poll.
    pub fn vote(&mut self, poll_id: &[u8; 16], choices: u16, now_secs: u64) -> CoreResult<()> {
        let poll = self.conv.poll(poll_id)?.ok_or(CoreError::UnknownMessage)?;
        if !valid_choice(&poll, choices) {
            return Err(CoreError::NotAllowed);
        }
        let me = self.my_fingerprint()?;
        if let Some(m) = self.conv.message(poll_id)? {
            let contact = self.contact(&m.contact_id)?;
            let content = Content::PollVote {
                group: None,
                poll_id: *poll_id,
                choices,
            };
            self.send_content(&contact, &content, now_secs)?;
        } else if let Some(group_id) = self.conv.group_of_message(poll_id)? {
            let g = self.conv.group(&group_id)?.ok_or(CoreError::UnknownGroup)?;
            if g.left || !g.members.iter().any(|(fp, _)| *fp == me) {
                return Err(CoreError::NotAllowed);
            }
            let content = Content::PollVote {
                group: Some(GroupRef {
                    group_id,
                    epoch: g.epoch,
                }),
                poll_id: *poll_id,
                choices,
            };
            let encoded = content.encode()?;
            for (fp, _) in g.members.iter().filter(|(fp, _)| *fp != me) {
                if let Some(c) = self.contact_by_fingerprint(fp)?.filter(|c| !c.blocked) {
                    self.send_content_bytes(&c, &encoded, now_secs)?;
                }
            }
        } else {
            return Err(CoreError::UnknownMessage);
        }
        self.conv
            .set_vote(poll_id, &me, choices, secs_i64(now_secs))?;
        Ok(())
    }

    /// The poll with current vote counts, or `None` if `poll_id` is not a poll.
    pub fn poll(&self, poll_id: &[u8; 16]) -> CoreResult<Option<PollView>> {
        let Some(p) = self.conv.poll(poll_id)? else {
            return Ok(None);
        };
        let me = self.my_fingerprint()?;
        let votes = self.conv.votes(poll_id)?;
        let mut counts = vec![0u32; p.options.len()];
        let mut mine = 0;
        for (voter, choices) in &votes {
            if *voter == me {
                mine = *choices;
            }
            for (i, c) in counts.iter_mut().enumerate() {
                if choices & (1 << i) != 0 {
                    *c += 1;
                }
            }
        }
        Ok(Some(PollView {
            question: p.question,
            options: p.options,
            multi: p.multi,
            counts,
            mine,
            voters: u32::try_from(votes.len()).unwrap_or(u32::MAX),
        }))
    }

    /// A poll from a contact. In a chat its id must be the message's own id; in
    /// a group it is held and accepted like any group message.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn on_poll(
        &mut self,
        contact: &Contact,
        message_id: [u8; 16],
        sent_at_ms: i64,
        group: Option<GroupRef>,
        poll_id: [u8; 16],
        row: PollRow,
        now_secs: u64,
    ) -> CoreResult<()> {
        match group {
            None => {
                if poll_id != message_id {
                    return Err(CoreError::NotAllowed);
                }
                self.conv.insert_message(&MessageRow {
                    message_id,
                    contact_id: contact.contact_id,
                    outgoing: false,
                    body: row.question.clone(),
                    created_at: secs_i64(now_secs),
                    state: MessageState::Received,
                    sent_at_ms,
                    ..Default::default()
                })?;
                if self.conv.message(&message_id)?.is_some_and(|m| !m.deleted) {
                    self.conv.put_poll(&poll_id, &row)?;
                }
            }
            Some(g) => {
                let p = PendingGroupMessage {
                    contact_id: contact.contact_id,
                    epoch: g.epoch,
                    message: GroupMessageRow {
                        group_msg_id: poll_id,
                        group_id: g.group_id,
                        author_fp: [0; 32],
                        outgoing: false,
                        body: row.question.clone(),
                        reply_to: None,
                        created_at: secs_i64(now_secs),
                        sent_at_ms,
                    },
                };
                self.conv.put_poll(&poll_id, &row)?;
                self.on_group_text(contact, p, now_secs)?;
            }
        }
        Ok(())
    }

    /// A vote counts only from someone who can see the poll: the chat partner,
    /// or a current member of the poll's group.
    pub(crate) fn on_poll_vote(
        &mut self,
        contact: &Contact,
        group: Option<GroupRef>,
        poll_id: [u8; 16],
        choices: u16,
        now_secs: u64,
    ) -> CoreResult<()> {
        let Some(poll) = self.conv.poll(&poll_id)? else {
            return Ok(());
        };
        if !valid_choice(&poll, choices) {
            return Ok(());
        }
        let voter = Self::fingerprint_of(contact)?;
        let allowed = match group {
            None => self
                .conv
                .message(&poll_id)?
                .is_some_and(|m| m.contact_id == contact.contact_id),
            Some(g) => {
                self.conv.group_of_message(&poll_id)? == Some(g.group_id)
                    && self.conv.group(&g.group_id)?.is_some_and(|row| {
                        !row.left && row.members.iter().any(|(fp, _)| *fp == voter)
                    })
            }
        };
        if allowed {
            self.conv
                .set_vote(&poll_id, &voter, choices, secs_i64(now_secs))?;
        }
        Ok(())
    }
}

/// Only bits for existing options; at most one bit unless the poll allows several.
fn valid_choice(poll: &PollRow, choices: u16) -> bool {
    let n = poll.options.len();
    let mask = if n >= 16 { u16::MAX } else { (1u16 << n) - 1 };
    choices & !mask == 0 && (poll.multi || choices.count_ones() <= 1)
}
