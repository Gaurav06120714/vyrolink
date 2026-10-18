//! Swift/Kotlin surface of the VYRO LINK core (UniFFI proc-macros).
//!
//! Transport is pull-based: the native Bluetooth layer passes received bytes to
//! [`VlNode::link_receive`] and writes back whatever it returns. Rust never calls
//! into Swift/Kotlin, so no foreign callbacks cross the boundary (ADR-022).
//! All methods are synchronous and thread-safe; callers should invoke them off
//! the main thread.

use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use vl_core::{
    ContactInfo, CoreError, GroupMessageRow, GroupRow, MessageRow, Node, NodeConfig, SendOptions,
};
use vl_routing::Policy;
use vl_store::MessageState;

uniffi::setup_scaffolding!();

#[derive(Debug, uniffi::Error)]
pub enum VlError {
    NoIdentity,
    UnknownContact,
    NoPairingInProgress,
    AlreadyPaired,
    OutOfOneTimeKeys,
    TextTooLong,
    UnknownLink,
    ContactBlocked,
    UnknownMessage,
    NotAllowed,
    InvalidReaction,
    InvalidTimer,
    /// The node was destroyed by [`VlNode::panic_wipe`]; open a new one.
    Wiped,
    UnknownGroup,
    NotGroupCreator,
    InvalidGroupSize,
    OwnPairingCode,
    InvalidAttachment,
    CannotResend,
    PaperUnavailable,
    NotForThisPhone,
    InvalidInput {
        reason: String,
    },
    Storage {
        reason: String,
    },
    Crypto {
        reason: String,
    },
}

impl std::fmt::Display for VlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for VlError {}

impl From<CoreError> for VlError {
    fn from(e: CoreError) -> Self {
        match e {
            CoreError::NoIdentity => Self::NoIdentity,
            CoreError::UnknownContact => Self::UnknownContact,
            CoreError::NoPairingInProgress => Self::NoPairingInProgress,
            CoreError::AlreadyPaired => Self::AlreadyPaired,
            CoreError::OutOfOneTimeKeys => Self::OutOfOneTimeKeys,
            CoreError::TextTooLong => Self::TextTooLong,
            CoreError::UnknownLink => Self::UnknownLink,
            CoreError::ContactBlocked => Self::ContactBlocked,
            CoreError::UnknownMessage => Self::UnknownMessage,
            CoreError::NotAllowed => Self::NotAllowed,
            CoreError::InvalidReaction => Self::InvalidReaction,
            CoreError::InvalidTimer => Self::InvalidTimer,
            CoreError::UnknownGroup => Self::UnknownGroup,
            CoreError::NotGroupCreator => Self::NotGroupCreator,
            CoreError::InvalidGroupSize => Self::InvalidGroupSize,
            CoreError::OwnPairingCode => Self::OwnPairingCode,
            CoreError::InvalidAttachment => Self::InvalidAttachment,
            CoreError::CannotResend => Self::CannotResend,
            CoreError::PaperUnavailable => Self::PaperUnavailable,
            CoreError::NotForThisPhone => Self::NotForThisPhone,
            CoreError::Wipe(kind) => Self::Storage {
                reason: format!("wipe could not remove a database file: {kind}"),
            },
            CoreError::Proto(p) => Self::InvalidInput {
                reason: p.to_string(),
            },
            CoreError::Crypto(c) => Self::Crypto {
                reason: c.to_string(),
            },
            CoreError::Db(d) => Self::Storage {
                reason: d.to_string(),
            },
            CoreError::Store(s) => Self::Storage {
                reason: s.to_string(),
            },
        }
    }
}

fn id16(bytes: &[u8]) -> Result<[u8; 16], VlError> {
    bytes.try_into().map_err(|_| VlError::InvalidInput {
        reason: "id must be 16 bytes".into(),
    })
}

fn key32(bytes: &[u8]) -> Result<[u8; 32], VlError> {
    bytes.try_into().map_err(|_| VlError::InvalidInput {
        reason: "key must be 32 bytes".into(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum DeliveryState {
    Queued,
    SentToNearby,
    AcceptedByRelay,
    Delivered,
    Read,
    Expired,
    DeliveryUnknown,
    Failed,
    Received,
}

impl From<MessageState> for DeliveryState {
    fn from(s: MessageState) -> Self {
        match s {
            MessageState::Queued => Self::Queued,
            MessageState::SentToNearby => Self::SentToNearby,
            MessageState::AcceptedByRelay => Self::AcceptedByRelay,
            MessageState::Acknowledged => Self::Delivered,
            MessageState::Read => Self::Read,
            MessageState::Expired => Self::Expired,
            MessageState::DeliveryUnknown => Self::DeliveryUnknown,
            MessageState::Failed => Self::Failed,
            MessageState::Received => Self::Received,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Contact {
    pub contact_id: Vec<u8>,
    pub display_name: String,
    pub fingerprint: Vec<u8>,
    /// 32-character ID code (A–Z, 0–9) derived from the identity key.
    pub id_code: String,
    pub verified: bool,
    pub blocked: bool,
    pub safety_number: String,
    /// Unix seconds this contact was last proven nearby (ADR-033).
    pub last_seen_secs: Option<i64>,
    /// Who introduced this contact (ADR-037); `None` if met in person.
    pub introduced_by: Option<String>,
}

impl From<ContactInfo> for Contact {
    fn from(c: ContactInfo) -> Self {
        Self {
            contact_id: c.contact_id.to_vec(),
            display_name: c.display_name,
            fingerprint: c.fingerprint.to_vec(),
            id_code: c.id_code,
            verified: c.verified,
            blocked: c.blocked,
            safety_number: c.safety_number,
            last_seen_secs: c.last_seen_secs,
            introduced_by: c.introduced_by,
        }
    }
}

/// `body` is the effective text (latest valid edit; empty when `deleted`).
#[derive(Debug, Clone, uniffi::Record)]
pub struct Message {
    pub message_id: Vec<u8>,
    pub contact_id: Vec<u8>,
    pub outgoing: bool,
    pub body: String,
    pub created_at_secs: i64,
    pub state: DeliveryState,
    pub reply_to: Option<Vec<u8>>,
    pub forwarded: bool,
    pub deleted: bool,
    pub edited: bool,
    pub disappears_at_secs: Option<i64>,
    pub pinned: bool,
    pub starred: bool,
    pub attachment: Option<AttachmentInfo>,
    pub location: Option<LocationInfo>,
    pub poll: Option<PollInfo>,
    pub intro: Option<IntroInfo>,
}

/// An introduction shown under its message (ADR-037). `state`: 0 asked,
/// 1 accepted, 2 declined, 3 connected, 4 failed (the card did not match).
#[derive(Debug, Clone, uniffi::Record)]
pub struct IntroInfo {
    pub intro_id: Vec<u8>,
    pub other_name: String,
    pub state: u8,
}

/// A shared location (ADR-034): degrees × 10⁷, accuracy in metres, fix time.
#[derive(Debug, Clone, uniffi::Record)]
pub struct LocationInfo {
    pub lat_e7: i32,
    pub lon_e7: i32,
    pub accuracy_m: u16,
    pub captured_ms: u64,
}

/// A poll with live counts (ADR-035). `mine` has bit i set for option i.
#[derive(Debug, Clone, uniffi::Record)]
pub struct PollInfo {
    pub question: String,
    pub options: Vec<String>,
    pub multi: bool,
    pub counts: Vec<u32>,
    pub mine: u16,
    pub voters: u32,
}

fn poll_info(n: &Node, id: &[u8; 16]) -> Option<PollInfo> {
    n.poll(id).ok().flatten().map(|p| PollInfo {
        question: p.question,
        options: p.options,
        multi: p.multi,
        counts: p.counts,
        mine: p.mine,
        voters: p.voters,
    })
}

/// Attachment summary (ADR-026). `kind` 1 = image, 2 = file. For my own
/// attachments `delivered_chunks` counts chunks with authenticated proof; the
/// bytes come from [`VlNode::attachment_data`].
#[derive(Debug, Clone, uniffi::Record)]
pub struct AttachmentInfo {
    pub kind: u8,
    pub name: String,
    pub total_len: u32,
    pub chunk_count: u32,
    pub received_chunks: u32,
    pub complete: bool,
    pub delivered_chunks: u32,
}

/// A message with its attachment summary, if any.
fn message(n: &Node, m: MessageRow) -> Message {
    let attachment = n.attachment(&m.message_id).ok().flatten().map(|a| {
        let delivered = n
            .attachment_delivery(&m.message_id)
            .map(|(d, _)| d)
            .unwrap_or(0);
        AttachmentInfo {
            kind: a.kind,
            name: a.name,
            total_len: a.total_len,
            chunk_count: vl_core::chunk_count(a.total_len),
            received_chunks: a.received_chunks,
            complete: a.complete,
            delivered_chunks: delivered,
        }
    });
    let location = n
        .location(&m.message_id)
        .ok()
        .flatten()
        .map(|l| LocationInfo {
            lat_e7: l.lat_e7,
            lon_e7: l.lon_e7,
            accuracy_m: l.accuracy_m,
            captured_ms: l.captured_ms,
        });
    let poll = poll_info(n, &m.message_id);
    let intro = n
        .intro_for_message(&m.message_id)
        .ok()
        .flatten()
        .map(|i| IntroInfo {
            intro_id: i.intro_id.to_vec(),
            other_name: i.other_name,
            state: i.state,
        });
    Message {
        attachment,
        location,
        poll,
        intro,
        ..Message::from(m)
    }
}

impl From<MessageRow> for Message {
    fn from(m: MessageRow) -> Self {
        Self {
            attachment: None,
            location: None,
            poll: None,
            intro: None,
            message_id: m.message_id.to_vec(),
            contact_id: m.contact_id.to_vec(),
            outgoing: m.outgoing,
            body: m.body,
            created_at_secs: m.created_at,
            state: m.state.into(),
            reply_to: m.reply_to.map(|r| r.to_vec()),
            forwarded: m.forwarded,
            deleted: m.deleted,
            edited: m.edited,
            disappears_at_secs: m.expire_at,
            pinned: m.pinned,
            starred: m.starred,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct ConversationSummary {
    pub contact: Contact,
    pub last: Option<Message>,
    pub unread: u32,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct MessageEvent {
    pub at_secs: i64,
    pub state: DeliveryState,
}

/// Counters only; no identifiers.
#[derive(Debug, Clone, uniffi::Record)]
pub struct NodeStats {
    pub open_links: u32,
    pub own_envelopes: u32,
    pub carried_envelopes: u32,
    pub carried_bytes: u64,
}

/// A QR symbol: `size` × `size` modules, row-major, 1 = dark. No quiet zone.
#[derive(Debug, Clone, uniffi::Record)]
pub struct QrMatrix {
    pub size: u32,
    pub modules: Vec<u8>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct GroupMemberInfo {
    pub fingerprint: Vec<u8>,
    pub name: String,
}

/// A small group (ADR-024). `left` is true after leaving or being removed.
#[derive(Debug, Clone, uniffi::Record)]
pub struct Group {
    pub group_id: Vec<u8>,
    pub name: String,
    pub epoch: u32,
    pub creator_fingerprint: Vec<u8>,
    pub left: bool,
    pub members: Vec<GroupMemberInfo>,
}

impl From<GroupRow> for Group {
    fn from(g: GroupRow) -> Self {
        Self {
            group_id: g.group_id.to_vec(),
            name: g.name,
            epoch: g.epoch,
            creator_fingerprint: g.creator_fp.to_vec(),
            left: g.left,
            members: g
                .members
                .into_iter()
                .map(|(fp, name)| GroupMemberInfo {
                    fingerprint: fp.to_vec(),
                    name,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct GroupMessage {
    pub group_msg_id: Vec<u8>,
    pub author_fingerprint: Vec<u8>,
    pub outgoing: bool,
    pub body: String,
    pub reply_to: Option<Vec<u8>>,
    pub created_at_secs: i64,
    pub poll: Option<PollInfo>,
}

impl From<GroupMessageRow> for GroupMessage {
    fn from(m: GroupMessageRow) -> Self {
        Self {
            group_msg_id: m.group_msg_id.to_vec(),
            author_fingerprint: m.author_fp.to_vec(),
            outgoing: m.outgoing,
            body: m.body,
            reply_to: m.reply_to.map(|r| r.to_vec()),
            created_at_secs: m.created_at,
            poll: None,
        }
    }
}

/// Pairwise copies of one of my group messages: delivered (with proof) / sent.
#[derive(Debug, Clone, uniffi::Record)]
pub struct GroupDelivery {
    pub delivered: u32,
    pub sent: u32,
}

/// A reaction in force on a message; `reaction` indexes [`reaction_set`] (1..=8).
#[derive(Debug, Clone, uniffi::Record)]
pub struct MessageReaction {
    pub from_me: bool,
    pub reaction: u8,
}

#[derive(Debug, uniffi::Object)]
pub struct VlNode {
    inner: Mutex<Option<Node>>,
}

#[uniffi::export]
impl VlNode {
    /// Opens both encrypted databases. Keys come from the platform key store and must be 32 bytes.
    #[uniffi::constructor]
    pub fn open(
        relay_db_path: String,
        relay_key: Vec<u8>,
        conversation_db_path: String,
        conversation_key: Vec<u8>,
    ) -> Result<Self, VlError> {
        let relay_db = PathBuf::from(relay_db_path);
        let conversation_db = PathBuf::from(conversation_db_path);
        let cfg = NodeConfig {
            relay_db: &relay_db,
            relay_key: key32(&relay_key)?,
            conversation_db: &conversation_db,
            conversation_key: key32(&conversation_key)?,
            policy: Policy::default_spray(),
        };
        Ok(Self {
            inner: Mutex::new(Some(Node::open(&cfg)?)),
        })
    }

    pub fn has_identity(&self) -> bool {
        self.lock().map(|n| n.has_identity()).unwrap_or(false)
    }

    pub fn create_identity(&self, now_secs: u64) -> Result<(), VlError> {
        Ok(self.lock()?.create_identity(now_secs)?)
    }

    /// Internet mailbox (ADR-039): sealed envelopes to upload, mine first.
    pub fn internet_outbox(&self, now_secs: u64) -> Result<Vec<Vec<u8>>, VlError> {
        Ok(self.lock()?.internet_outbox(now_secs)?)
    }

    pub fn internet_uploaded(&self, envelopes: Vec<Vec<u8>>, now_secs: u64) -> Result<(), VlError> {
        Ok(self.lock()?.internet_uploaded(&envelopes, now_secs)?)
    }

    pub fn internet_tags(&self) -> Result<Vec<Vec<u8>>, VlError> {
        Ok(self
            .lock()?
            .internet_tags()?
            .iter()
            .map(|t| t.to_vec())
            .collect())
    }

    /// True if the envelope was new mail or a new receipt for this phone.
    pub fn receive_internet(&self, envelope: Vec<u8>, now_secs: u64) -> Result<bool, VlError> {
        Ok(self.lock()?.receive_internet(&envelope, now_secs)?)
    }

    /// Long-term keys only, for Keychain / Block Store (ADR-038). Never log.
    pub fn identity_backup(&self) -> Result<Vec<u8>, VlError> {
        Ok(self.lock()?.identity_backup()?.to_vec())
    }

    pub fn restore_identity(&self, backup: Vec<u8>, now_secs: u64) -> Result<(), VlError> {
        Ok(self.lock()?.restore_identity(&backup, now_secs)?)
    }

    pub fn pairing_card(&self, display_name: String, now_secs: u64) -> Result<Vec<u8>, VlError> {
        Ok(self.lock()?.pairing_card(&display_name, now_secs)?)
    }

    pub fn complete_pairing(&self, their_card: Vec<u8>, now_secs: u64) -> Result<Contact, VlError> {
        Ok(self.lock()?.complete_pairing(&their_card, now_secs)?.into())
    }

    /// The pairing card as QR text (`VL1:` + Base45). Keep showing the same
    /// code until pairing completes.
    pub fn pairing_code(&self, display_name: String, now_secs: u64) -> Result<String, VlError> {
        Ok(self.lock()?.pairing_code(&display_name, now_secs)?)
    }

    /// Completes pairing from scanned QR text; call [`Self::pairing_code`] first.
    pub fn complete_pairing_code(&self, code: String, now_secs: u64) -> Result<Contact, VlError> {
        Ok(self.lock()?.complete_pairing_code(&code, now_secs)?.into())
    }

    /// This device's 32-character ID code (unique per identity, offline).
    pub fn my_id_code(&self) -> Result<String, VlError> {
        Ok(self.lock()?.my_id_code()?)
    }

    pub fn rename_contact(&self, contact_id: Vec<u8>, name: String) -> Result<(), VlError> {
        Ok(self.lock()?.rename_contact(&id16(&contact_id)?, &name)?)
    }

    /// Deletes the contact and the whole conversation with them.
    pub fn delete_contact(&self, contact_id: Vec<u8>) -> Result<(), VlError> {
        Ok(self.lock()?.delete_contact(&id16(&contact_id)?)?)
    }

    /// Re-encrypts an expired or unconfirmed message and queues it again.
    pub fn resend(&self, message_id: Vec<u8>, now_secs: u64) -> Result<(), VlError> {
        Ok(self.lock()?.resend(&id16(&message_id)?, now_secs)?)
    }

    /// Relay storage for others' mail, clamped to 1 MiB ..= 100 MiB.
    pub fn set_carry_cap(&self, bytes: u64) -> Result<(), VlError> {
        self.lock()?.set_carry_cap(bytes);
        Ok(())
    }

    pub fn carry_cap(&self) -> Result<u64, VlError> {
        Ok(self.lock()?.carry_cap())
    }

    /// Sends an image (`kind` 1, ≤ 16 KiB) or file (`kind` 2, ≤ 64 KiB).
    #[allow(clippy::too_many_arguments)]
    pub fn send_attachment(
        &self,
        contact_id: Vec<u8>,
        kind: u8,
        name: String,
        data: Vec<u8>,
        caption: String,
        reply_to: Option<Vec<u8>>,
        now_secs: u64,
    ) -> Result<Vec<u8>, VlError> {
        let reply = reply_to.as_deref().map(id16).transpose()?;
        Ok(self
            .lock()?
            .send_attachment(
                &id16(&contact_id)?,
                kind,
                &name,
                &data,
                &caption,
                reply,
                now_secs,
            )?
            .to_vec())
    }

    /// The bytes of a complete attachment (received or my own).
    pub fn attachment_data(&self, message_id: Vec<u8>) -> Result<Option<Vec<u8>>, VlError> {
        Ok(self.lock()?.attachment_data(&id16(&message_id)?)?)
    }

    /// This device's identity key, for display.
    pub fn my_fingerprint(&self) -> Result<Vec<u8>, VlError> {
        Ok(self.lock()?.my_fingerprint()?.to_vec())
    }

    /// Chat list: every contact with its newest message, most recent first.
    pub fn conversation_summaries(&self) -> Result<Vec<ConversationSummary>, VlError> {
        let n = self.lock()?;
        Ok(n.conversation_summaries()?
            .into_iter()
            .map(|s| ConversationSummary {
                contact: s.contact.into(),
                last: s.last.map(|m| message(&n, m)),
                unread: s.unread,
            })
            .collect())
    }

    /// My outgoing messages without delivery proof yet, newest first.
    pub fn pending_outgoing(&self) -> Result<Vec<Message>, VlError> {
        let n = self.lock()?;
        Ok(n.pending_outgoing()?
            .into_iter()
            .map(|m| message(&n, m))
            .collect())
    }

    /// State history of one message, oldest first.
    pub fn message_events(&self, message_id: Vec<u8>) -> Result<Vec<MessageEvent>, VlError> {
        Ok(self
            .lock()?
            .message_events(&id16(&message_id)?)?
            .into_iter()
            .map(|(at, state)| MessageEvent {
                at_secs: at,
                state: state.into(),
            })
            .collect())
    }

    /// Local settings stored in the encrypted conversation database.
    pub fn setting(&self, key: String) -> Result<Option<String>, VlError> {
        Ok(self.lock()?.setting(&key)?)
    }

    /// `None` removes the key.
    pub fn set_setting(&self, key: String, value: Option<String>) -> Result<(), VlError> {
        Ok(self.lock()?.set_setting(&key, value.as_deref())?)
    }

    pub fn stats(&self) -> Result<NodeStats, VlError> {
        let s = self.lock()?.stats()?;
        Ok(NodeStats {
            open_links: s.open_links,
            own_envelopes: s.own_envelopes,
            carried_envelopes: s.carried_envelopes,
            carried_bytes: s.carried_bytes,
        })
    }

    pub fn contacts(&self) -> Result<Vec<Contact>, VlError> {
        Ok(self
            .lock()?
            .contacts()?
            .into_iter()
            .map(Contact::from)
            .collect())
    }

    pub fn set_verified(&self, contact_id: Vec<u8>, verified: bool) -> Result<(), VlError> {
        Ok(self.lock()?.set_verified(&id16(&contact_id)?, verified)?)
    }

    pub fn send_text(
        &self,
        contact_id: Vec<u8>,
        text: String,
        now_secs: u64,
    ) -> Result<Vec<u8>, VlError> {
        Ok(self
            .lock()?
            .send_text(&id16(&contact_id)?, &text, now_secs)?
            .to_vec())
    }

    /// Sends text as a reply (`reply_to` = target message id) and/or marked forwarded.
    pub fn send_message(
        &self,
        contact_id: Vec<u8>,
        text: String,
        reply_to: Option<Vec<u8>>,
        forwarded: bool,
        now_secs: u64,
    ) -> Result<Vec<u8>, VlError> {
        let opts = SendOptions {
            reply_to: reply_to.as_deref().map(id16).transpose()?,
            forwarded,
        };
        Ok(self
            .lock()?
            .send_message(&id16(&contact_id)?, &text, opts, now_secs)?
            .to_vec())
    }

    pub fn conversation(&self, contact_id: Vec<u8>, limit: u32) -> Result<Vec<Message>, VlError> {
        let n = self.lock()?;
        Ok(n.conversation(&id16(&contact_id)?, limit)?
            .into_iter()
            .map(|m| message(&n, m))
            .collect())
    }

    pub fn message(&self, message_id: Vec<u8>) -> Result<Option<Message>, VlError> {
        let n = self.lock()?;
        Ok(n.message(&id16(&message_id)?)?.map(|m| message(&n, m)))
    }

    /// `reaction`: 0 removes mine, 1..=8 index [`reaction_set`].
    pub fn react(
        &self,
        contact_id: Vec<u8>,
        message_id: Vec<u8>,
        reaction: u8,
        now_secs: u64,
    ) -> Result<(), VlError> {
        Ok(self
            .lock()?
            .react(&id16(&contact_id)?, &id16(&message_id)?, reaction, now_secs)?)
    }

    pub fn reactions(&self, message_id: Vec<u8>) -> Result<Vec<MessageReaction>, VlError> {
        Ok(self
            .lock()?
            .reactions(&id16(&message_id)?)?
            .into_iter()
            .map(|r| MessageReaction {
                from_me: r.from_me,
                reaction: r.reaction,
            })
            .collect())
    }

    pub fn edit_message(
        &self,
        contact_id: Vec<u8>,
        message_id: Vec<u8>,
        text: String,
        now_secs: u64,
    ) -> Result<(), VlError> {
        Ok(self
            .lock()?
            .edit_message(&id16(&contact_id)?, &id16(&message_id)?, &text, now_secs)?)
    }

    /// Requests deletion on the contact's device; it cannot recall copies already read or captured.
    pub fn delete_for_everyone(
        &self,
        contact_id: Vec<u8>,
        message_id: Vec<u8>,
        now_secs: u64,
    ) -> Result<(), VlError> {
        Ok(self
            .lock()?
            .delete_for_everyone(&id16(&contact_id)?, &id16(&message_id)?, now_secs)?)
    }

    pub fn delete_for_me(&self, message_id: Vec<u8>, now_secs: u64) -> Result<(), VlError> {
        Ok(self.lock()?.delete_for_me(&id16(&message_id)?, now_secs)?)
    }

    /// 0 turns disappearing messages off; otherwise 60 s ..= 365 days.
    pub fn set_disappearing_timer(
        &self,
        contact_id: Vec<u8>,
        timer_secs: u32,
        now_secs: u64,
    ) -> Result<(), VlError> {
        Ok(self
            .lock()?
            .set_disappearing_timer(&id16(&contact_id)?, timer_secs, now_secs)?)
    }

    pub fn disappearing_timer(&self, contact_id: Vec<u8>) -> Result<u32, VlError> {
        Ok(self.lock()?.disappearing_timer(&id16(&contact_id)?)?)
    }

    pub fn mark_read(&self, contact_id: Vec<u8>, now_secs: u64) -> Result<u32, VlError> {
        Ok(self.lock()?.mark_read(&id16(&contact_id)?, now_secs)?)
    }

    /// Searches effective message text (word prefixes); `contact_id` narrows to one conversation.
    pub fn search(
        &self,
        query: String,
        contact_id: Option<Vec<u8>>,
        limit: u32,
    ) -> Result<Vec<Message>, VlError> {
        let contact = contact_id.as_deref().map(id16).transpose()?;
        let n = self.lock()?;
        Ok(n.search(&query, contact.as_ref(), limit)?
            .into_iter()
            .map(|m| message(&n, m))
            .collect())
    }

    pub fn save_draft(
        &self,
        contact_id: Vec<u8>,
        text: String,
        now_secs: u64,
    ) -> Result<(), VlError> {
        Ok(self
            .lock()?
            .save_draft(&id16(&contact_id)?, &text, now_secs)?)
    }

    pub fn draft(&self, contact_id: Vec<u8>) -> Result<Option<String>, VlError> {
        Ok(self.lock()?.draft(&id16(&contact_id)?)?)
    }

    pub fn set_blocked(
        &self,
        contact_id: Vec<u8>,
        blocked: bool,
        now_secs: u64,
    ) -> Result<(), VlError> {
        Ok(self
            .lock()?
            .set_blocked(&id16(&contact_id)?, blocked, now_secs)?)
    }

    pub fn set_pinned(&self, message_id: Vec<u8>, pinned: bool) -> Result<bool, VlError> {
        Ok(self.lock()?.set_pinned(&id16(&message_id)?, pinned)?)
    }

    pub fn set_starred(&self, message_id: Vec<u8>, starred: bool) -> Result<bool, VlError> {
        Ok(self.lock()?.set_starred(&id16(&message_id)?, starred)?)
    }

    /// Pinned (`pinned = true`) or starred messages, newest first.
    pub fn flagged(
        &self,
        contact_id: Option<Vec<u8>>,
        pinned: bool,
    ) -> Result<Vec<Message>, VlError> {
        let contact = contact_id.as_deref().map(id16).transpose()?;
        let n = self.lock()?;
        Ok(n.flagged(contact.as_ref(), pinned)?
            .into_iter()
            .map(|m| message(&n, m))
            .collect())
    }

    pub fn maintain(&self, now_secs: u64) -> Result<(), VlError> {
        Ok(self.lock()?.maintain(now_secs)?)
    }

    /// Call when a BLE connection is established. Returns the link id; send the
    /// returned bytes (possibly empty) immediately.
    pub fn link_open(&self, initiator: bool) -> Result<LinkStart, VlError> {
        let (id, bytes) = self.lock()?.link_open(initiator)?;
        Ok(LinkStart {
            link_id: id,
            bytes_to_send: bytes,
        })
    }

    /// Feed received bytes; write the returned bytes back. An error means the
    /// link is closed and the BLE connection should be dropped.
    pub fn link_receive(
        &self,
        link_id: u64,
        bytes: Vec<u8>,
        now_secs: u64,
    ) -> Result<Vec<u8>, VlError> {
        Ok(self.lock()?.link_receive(link_id, &bytes, now_secs)?)
    }

    pub fn link_close(&self, link_id: u64) {
        if let Ok(mut n) = self.lock() {
            n.link_close(link_id);
        }
    }

    /// Unblocked contacts seen over Bluetooth within `within_secs` of `now_secs`.
    pub fn nearby_contacts(
        &self,
        now_secs: u64,
        within_secs: u64,
    ) -> Result<Vec<NearbyContact>, VlError> {
        Ok(self
            .lock()?
            .nearby_contacts(now_secs, within_secs)?
            .into_iter()
            .map(|(id, seen)| NearbyContact {
                contact_id: id.to_vec(),
                seen_secs: seen,
            })
            .collect())
    }

    /// Contacts that came into range since the last call (for one generic
    /// notification that never names them).
    pub fn take_nearby_arrivals(&self) -> u32 {
        self.lock()
            .map(|mut n| n.take_nearby_arrivals())
            .unwrap_or(0)
    }

    /// Seconds to wait before syncing with the same nearby phone again (1 ..= 45, default 5).
    pub fn resync_secs(&self) -> u32 {
        self.lock()
            .and_then(|n| n.resync_secs().map_err(Into::into))
            .unwrap_or(vl_core::RESYNC_DEFAULT_SECS)
    }

    /// Stores the interval (clamped) and returns the stored value.
    pub fn set_resync_secs(&self, secs: u32) -> Result<u32, VlError> {
        Ok(self.lock()?.set_resync_secs(secs)?)
    }

    pub fn link_is_open(&self, link_id: u64) -> bool {
        self.lock()
            .map(|n| n.link_is_open(link_id))
            .unwrap_or(false)
    }

    /// Creates a group with these contacts (2 ..= 8 members including me).
    pub fn create_group(
        &self,
        name: String,
        my_name: String,
        contact_ids: Vec<Vec<u8>>,
        now_secs: u64,
    ) -> Result<Vec<u8>, VlError> {
        let ids = contact_ids
            .iter()
            .map(|c| id16(c))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self
            .lock()?
            .create_group(&name, &my_name, &ids, now_secs)?
            .to_vec())
    }

    /// Replaces the members (creator only).
    pub fn set_group_members(
        &self,
        group_id: Vec<u8>,
        my_name: String,
        contact_ids: Vec<Vec<u8>>,
        now_secs: u64,
    ) -> Result<(), VlError> {
        let ids = contact_ids
            .iter()
            .map(|c| id16(c))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self
            .lock()?
            .set_group_members(&id16(&group_id)?, &my_name, &ids, now_secs)?)
    }

    pub fn leave_group(&self, group_id: Vec<u8>, now_secs: u64) -> Result<(), VlError> {
        Ok(self.lock()?.leave_group(&id16(&group_id)?, now_secs)?)
    }

    pub fn groups(&self) -> Result<Vec<Group>, VlError> {
        Ok(self
            .lock()?
            .groups()?
            .into_iter()
            .map(Group::from)
            .collect())
    }

    pub fn group_messages(
        &self,
        group_id: Vec<u8>,
        limit: u32,
    ) -> Result<Vec<GroupMessage>, VlError> {
        let n = self.lock()?;
        Ok(n.group_messages(&id16(&group_id)?, limit)?
            .into_iter()
            .map(|m| {
                let poll = poll_info(&n, &m.group_msg_id);
                GroupMessage {
                    poll,
                    ..GroupMessage::from(m)
                }
            })
            .collect())
    }

    /// Shares this phone's position with a contact (ADR-034).
    pub fn send_location(
        &self,
        contact_id: Vec<u8>,
        lat_e7: i32,
        lon_e7: i32,
        accuracy_m: u16,
        captured_ms: u64,
        now_secs: u64,
    ) -> Result<Vec<u8>, VlError> {
        Ok(self
            .lock()?
            .send_location(
                &id16(&contact_id)?,
                lat_e7,
                lon_e7,
                accuracy_m,
                captured_ms,
                now_secs,
            )?
            .to_vec())
    }

    /// Starts a poll in a chat (ADR-035); returns the poll's message id.
    pub fn send_poll(
        &self,
        contact_id: Vec<u8>,
        question: String,
        options: Vec<String>,
        multi: bool,
        now_secs: u64,
    ) -> Result<Vec<u8>, VlError> {
        Ok(self
            .lock()?
            .send_poll(&id16(&contact_id)?, &question, &options, multi, now_secs)?
            .to_vec())
    }

    /// Starts a poll in a group; returns the poll's group message id.
    pub fn send_group_poll(
        &self,
        group_id: Vec<u8>,
        question: String,
        options: Vec<String>,
        multi: bool,
        now_secs: u64,
    ) -> Result<Vec<u8>, VlError> {
        Ok(self
            .lock()?
            .send_group_poll(&id16(&group_id)?, &question, &options, multi, now_secs)?
            .to_vec())
    }

    /// Introduces two of my contacts to each other (ADR-037).
    pub fn introduce(&self, a: Vec<u8>, b: Vec<u8>, now_secs: u64) -> Result<Vec<u8>, VlError> {
        Ok(self
            .lock()?
            .introduce(&id16(&a)?, &id16(&b)?, now_secs)?
            .to_vec())
    }

    /// Accepts an introduction; `my_name` goes on the fresh card.
    pub fn accept_intro(
        &self,
        intro_id: Vec<u8>,
        my_name: String,
        now_secs: u64,
    ) -> Result<(), VlError> {
        Ok(self
            .lock()?
            .accept_intro(&id16(&intro_id)?, &my_name, now_secs)?)
    }

    pub fn decline_intro(&self, intro_id: Vec<u8>) -> Result<(), VlError> {
        Ok(self.lock()?.decline_intro(&id16(&intro_id)?)?)
    }

    /// QR text for one of my undelivered messages (ADR-036).
    pub fn paper_message(&self, message_id: Vec<u8>) -> Result<String, VlError> {
        Ok(self.lock()?.paper_message(&id16(&message_id)?)?)
    }

    /// A scanned message code; returns the contact id it came from.
    pub fn receive_paper(&self, code: String, now_secs: u64) -> Result<Vec<u8>, VlError> {
        Ok(self.lock()?.receive_paper(&code, now_secs)?.to_vec())
    }

    /// Sets my whole choice for a poll (bit i = option i; 0 withdraws).
    pub fn vote(&self, poll_id: Vec<u8>, choices: u16, now_secs: u64) -> Result<(), VlError> {
        Ok(self.lock()?.vote(&id16(&poll_id)?, choices, now_secs)?)
    }

    pub fn send_group_message(
        &self,
        group_id: Vec<u8>,
        text: String,
        reply_to: Option<Vec<u8>>,
        now_secs: u64,
    ) -> Result<Vec<u8>, VlError> {
        let reply = reply_to.as_deref().map(id16).transpose()?;
        Ok(self
            .lock()?
            .send_group_message(&id16(&group_id)?, &text, reply, now_secs)?
            .to_vec())
    }

    pub fn group_delivery(&self, group_msg_id: Vec<u8>) -> Result<GroupDelivery, VlError> {
        let (delivered, sent) = self.lock()?.group_delivery(&id16(&group_msg_id)?)?;
        Ok(GroupDelivery { delivered, sent })
    }

    /// Panic wipe. **Delete both database keys from the Keychain/Keystore
    /// first** (that is what makes the data unrecoverable on flash storage),
    /// then call this: it closes the databases, drops in-memory keys and
    /// sessions, and deletes the database files. Every later call on this
    /// object returns `Wiped`. Works even if an earlier panic poisoned the lock.
    pub fn panic_wipe(&self) -> Result<(), VlError> {
        let mut slot = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let node = slot.take().ok_or(VlError::Wiped)?;
        Ok(node.panic_wipe()?)
    }
}

/// Exclusive access to a live (not wiped) node.
struct NodeGuard<'a>(MutexGuard<'a, Option<Node>>);

impl Deref for NodeGuard<'_> {
    type Target = Node;
    fn deref(&self) -> &Node {
        match self.0.as_ref() {
            Some(n) => n,
            None => unreachable!("NodeGuard is only built around a live node"),
        }
    }
}

impl DerefMut for NodeGuard<'_> {
    fn deref_mut(&mut self) -> &mut Node {
        match self.0.as_mut() {
            Some(n) => n,
            None => unreachable!("NodeGuard is only built around a live node"),
        }
    }
}

impl VlNode {
    fn lock(&self) -> Result<NodeGuard<'_>, VlError> {
        let guard = self.inner.lock().map_err(|_| VlError::Storage {
            reason: "node lock poisoned by an earlier panic".into(),
        })?;
        if guard.is_none() {
            return Err(VlError::Wiped);
        }
        Ok(NodeGuard(guard))
    }
}

/// A paired contact seen in Bluetooth range (ADR-029). Kept in memory only.
#[derive(Debug, Clone, uniffi::Record)]
pub struct NearbyContact {
    pub contact_id: Vec<u8>,
    pub seen_secs: u64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct LinkStart {
    pub link_id: u64,
    pub bytes_to_send: Vec<u8>,
}

/// Maximum UTF-8 bytes per message.
#[uniffi::export]
pub fn max_text_bytes() -> u32 {
    u32::try_from(vl_core::MAX_TEXT_BYTES).unwrap_or(u32::MAX)
}

/// The fixed reaction set; reaction n (1..=8) is element n − 1.
#[uniffi::export]
pub fn reaction_set() -> Vec<String> {
    vl_core::REACTIONS.iter().map(|r| (*r).to_owned()).collect()
}

/// Encodes text (a pairing code) as a QR symbol with medium error correction.
#[uniffi::export]
pub fn qr_matrix(text: String) -> Result<QrMatrix, VlError> {
    let code = qrcode::QrCode::with_error_correction_level(text.as_bytes(), qrcode::EcLevel::M)
        .map_err(|e| VlError::InvalidInput {
            reason: e.to_string(),
        })?;
    let size = u32::try_from(code.width()).unwrap_or(0);
    let modules = code
        .to_colors()
        .into_iter()
        .map(|c| u8::from(c == qrcode::Color::Dark))
        .collect();
    Ok(QrMatrix { size, modules })
}

/// Removes both databases and their side files without opening them (for a
/// reset when the keys are gone). Delete the keys from the key store first.
#[uniffi::export]
pub fn remove_database_files(
    relay_db_path: String,
    conversation_db_path: String,
) -> Result<(), VlError> {
    Ok(vl_core::remove_database_files(&[
        PathBuf::from(relay_db_path),
        PathBuf::from(conversation_db_path),
    ])?)
}
