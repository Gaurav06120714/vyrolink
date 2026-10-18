//! A VYRO LINK node: the single entry point the native hosts call (Doc 18 §6.1).
//! It owns the identity, both encrypted databases and every live link session,
//! and turns routing events into message-state changes. Time is always passed
//! in by the caller so behaviour is deterministic and testable.

mod internet;
mod intro;
mod link;
mod paper;
pub use intro::{INTRO_ACCEPTED, INTRO_ASKED, INTRO_CONNECTED, INTRO_DECLINED, INTRO_FAILED};
mod share;
pub use internet::INTERNET_BATCH;
pub use share::{LOCATION_BODY, PollView};

pub use link::{ARRIVAL_GAP_SECS, LinkId};

use link::Link;
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use vl_crypto::{
    CryptoError, EPOCH_SECONDS, Identity, OlmPayload, Outgoing, PairKeys, SendParams,
    build_attachment_chunk, build_content_bytes, build_content_bytes_with_id, build_receipt,
    decrypt_olm, epoch_of, id_code, olm_payload, open_chunk, open_from, random_array,
    safety_number, seal_chunk, sha256, verify_receipt,
};
pub use vl_proto::chunk_count_for as chunk_count;
use vl_proto::chunk_count_for;
pub use vl_proto::{
    ATTACHMENT_FILE, ATTACHMENT_IMAGE, Content, GroupMember, MAX_ATTACHMENT_NAME,
    MAX_CAPTION_BYTES, MAX_EDIT_SEQ, MAX_FILE_BYTES, MAX_GROUP_MEMBERS, MAX_IMAGE_BYTES,
    MAX_TIMER_SECS, MIN_TIMER_SECS, REACTIONS,
};
use vl_proto::{CHUNK_BYTES, ChunkPayload};
use vl_proto::{
    Envelope, InnerKind, PairingCard, ProtoError, VaccineItem, decode_pairing_code,
    encode_pairing_code,
};
use vl_routing::{Event, Policy, Store, StoreError, Stored, maintain};
pub use vl_store::{AttachmentRow, GroupMessageRow, GroupRow, MessageRow, MessageState, Reaction};
use vl_store::{
    Contact, ConversationDb, DELETE_WINDOW_MS, DbError, EDIT_WINDOW_MS, EventMeta, GroupApply,
    PendingGroupMessage, RelayDb,
};
use vodozemac::olm::{Session, SessionPickle};
use zeroize::Zeroizing;

pub const MAX_TEXT_BYTES: usize = 1700;
/// A contact's timestamp further ahead of our clock than this is not absorbed
/// into the hybrid logical clock, so a wrong or hostile clock cannot drag ours
/// forward without bound (Kulkarni et al., HLC, 2014).
pub const MAX_CLOCK_DRIFT_MS: i64 = 60 * 60 * 1000;
const ONE_TIME_KEYS_PER_CARD: usize = 5;

#[derive(Debug)]
pub enum CoreError {
    Db(DbError),
    Store(StoreError),
    Crypto(CryptoError),
    Proto(vl_proto::ProtoError),
    NoIdentity,
    UnknownContact,
    NoPairingInProgress,
    AlreadyPaired,
    OutOfOneTimeKeys,
    TextTooLong,
    UnknownLink,
    ContactBlocked,
    UnknownMessage,
    /// Edit/delete refused: not the author, window passed, edit limit reached or message deleted.
    NotAllowed,
    InvalidReaction,
    InvalidTimer,
    /// A database file could not be removed during a panic wipe.
    Wipe(std::io::ErrorKind),
    UnknownGroup,
    /// Only the group's creator may change its members.
    NotGroupCreator,
    /// A group needs 2 ..= MAX_GROUP_MEMBERS members including me.
    InvalidGroupSize,
    /// The scanned pairing code is this device's own.
    OwnPairingCode,
    /// Attachment too large, empty, or with an over-long name or caption.
    InvalidAttachment,
    /// Resend is only possible for my messages that expired or whose delivery is unknown.
    CannotResend,
    /// A QR paper message can't be made: delivered or expired already, or too long (ADR-036).
    PaperUnavailable,
    /// The scanned message code is for someone else, or expired (ADR-036).
    NotForThisPhone,
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Db(e) => write!(f, "{e}"),
            Self::Store(e) => write!(f, "{e}"),
            Self::Crypto(e) => write!(f, "{e}"),
            Self::Proto(e) => write!(f, "{e}"),
            Self::NoIdentity => f.write_str("no identity on this device"),
            Self::UnknownContact => f.write_str("unknown contact"),
            Self::NoPairingInProgress => f.write_str("show your pairing code first"),
            Self::AlreadyPaired => f.write_str("already paired with this contact"),
            Self::OutOfOneTimeKeys => f.write_str("contact's one-time keys exhausted; pair again"),
            Self::TextTooLong => write!(f, "messages are limited to {MAX_TEXT_BYTES} bytes"),
            Self::UnknownLink => f.write_str("unknown link"),
            Self::ContactBlocked => f.write_str("contact is blocked"),
            Self::UnknownMessage => f.write_str("unknown message"),
            Self::NotAllowed => f.write_str("not allowed for this message"),
            Self::InvalidReaction => f.write_str("unknown reaction"),
            Self::InvalidTimer => f.write_str("timer must be off or between 1 minute and 1 year"),
            Self::Wipe(kind) => write!(f, "wipe could not remove a database file: {kind}"),
            Self::UnknownGroup => f.write_str("unknown group"),
            Self::NotGroupCreator => f.write_str("only the group's creator can change members"),
            Self::InvalidGroupSize => {
                write!(
                    f,
                    "a group has 2 to {MAX_GROUP_MEMBERS} members including you"
                )
            }
            Self::OwnPairingCode => f.write_str("this is your own pairing code"),
            Self::InvalidAttachment => f.write_str("attachment too large or invalid"),
            Self::CannotResend => f.write_str("only expired or unconfirmed messages can be resent"),
            Self::PaperUnavailable => f.write_str("this message can't be shown as a code"),
            Self::NotForThisPhone => f.write_str("this message code is not for this phone"),
        }
    }
}

impl std::error::Error for CoreError {}

macro_rules! from_err {
    ($($t:ty => $v:ident),*) => {$(impl From<$t> for CoreError { fn from(e: $t) -> Self { Self::$v(e) } })*};
}
from_err!(DbError => Db, StoreError => Store, CryptoError => Crypto, vl_proto::ProtoError => Proto);

pub type CoreResult<T> = Result<T, CoreError>;

/// What a contact looks like to the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactInfo {
    pub contact_id: [u8; 16],
    pub display_name: String,
    /// The contact's identity key; group rosters identify members by it.
    pub fingerprint: [u8; 32],
    /// 32-character ID code derived from `fingerprint` (display and comparison only).
    pub id_code: String,
    pub verified: bool,
    pub blocked: bool,
    pub safety_number: String,
    /// Unix seconds when a Bluetooth link last proved this contact was nearby
    /// (ADR-033); `None` if never.
    pub last_seen_secs: Option<i64>,
    /// Name of the contact who introduced this one (ADR-037); `None` if met in person.
    pub introduced_by: Option<String>,
}

/// One chat-list row: a contact, its newest message and unread count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationSummary {
    pub contact: ContactInfo,
    pub last: Option<MessageRow>,
    pub unread: u32,
}

/// Counters for the Network screen. No identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NodeStats {
    pub open_links: u32,
    pub own_envelopes: u32,
    pub carried_envelopes: u32,
    pub carried_bytes: u64,
}

/// Optional attributes of an outgoing text message.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SendOptions {
    pub reply_to: Option<[u8; 16]>,
    pub forwarded: bool,
}

/// Paths and keys supplied by the native host (keys come from Keychain / Keystore).
#[derive(Debug, Clone)]
pub struct NodeConfig<'a> {
    pub relay_db: &'a Path,
    pub relay_key: [u8; 32],
    pub conversation_db: &'a Path,
    pub conversation_key: [u8; 32],
    pub policy: Policy,
}

pub struct Node {
    identity: Option<Identity>,
    pickle_key: Zeroizing<[u8; 32]>,
    db_paths: [PathBuf; 2],
    relay: RelayDb,
    conv: ConversationDb,
    policy: Policy,
    pending_card: Option<PairingCard>,
    links: HashMap<LinkId, Link>,
    next_link: u64,
    /// Contact id → when a link last proved that contact was in Bluetooth range
    /// (ADR-029). Memory only: never written to disk.
    nearby: HashMap<[u8; 16], u64>,
    /// Contacts that came into range after being away (see `take_nearby_arrivals`).
    nearby_arrivals: u32,
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Node")
            .field("links", &self.links.len())
            .finish_non_exhaustive()
    }
}

/// Re-sync interval (ADR-032).
pub const RESYNC_DEFAULT_SECS: u32 = 5;
pub const RESYNC_MIN_SECS: u32 = 1;
pub const RESYNC_MAX_SECS: u32 = 45;
const RESYNC_SETTING: &str = "relay.resync_secs";

fn now_min(now_secs: u64) -> u32 {
    u32::try_from(now_secs / 60).unwrap_or(u32::MAX)
}

fn secs_i64(now_secs: u64) -> i64 {
    i64::try_from(now_secs).unwrap_or(i64::MAX)
}

fn ms_i64(now_secs: u64) -> i64 {
    secs_i64(now_secs).saturating_mul(1000)
}

impl Node {
    pub fn open(cfg: &NodeConfig<'_>) -> CoreResult<Self> {
        let relay = RelayDb::open(cfg.relay_db, &cfg.relay_key)?;
        let conv = ConversationDb::open(cfg.conversation_db, &cfg.conversation_key)?;
        let identity = match conv.identity()? {
            Some((account, addressing)) => Some(Identity::from_pickle(
                &account,
                addressing,
                &cfg.conversation_key,
            )?),
            None => None,
        };
        Ok(Self {
            identity,
            pickle_key: Zeroizing::new(cfg.conversation_key),
            db_paths: [
                cfg.relay_db.to_path_buf(),
                cfg.conversation_db.to_path_buf(),
            ],
            relay,
            conv,
            policy: cfg.policy,
            pending_card: None,
            links: HashMap::new(),
            next_link: 0,
            nearby: HashMap::new(),
            nearby_arrivals: 0,
        })
    }

    pub fn has_identity(&self) -> bool {
        self.identity.is_some()
    }

    pub fn create_identity(&mut self, now_secs: u64) -> CoreResult<()> {
        let identity = Identity::generate()?;
        self.persist_identity_with(&identity, now_secs)?;
        self.identity = Some(identity);
        Ok(())
    }

    /// This phone's long-term keys for the platform store that survives an
    /// uninstall (ADR-038). Secret: hosts must never log it.
    pub fn identity_backup(&self) -> CoreResult<Zeroizing<Vec<u8>>> {
        Ok(self
            .identity
            .as_ref()
            .ok_or(CoreError::NoIdentity)?
            .backup()?)
    }

    /// Brings back this phone's identity after a reinstall (ADR-038): same ID,
    /// QR identity and safety numbers. Chats and sessions are not in the
    /// backup, so contacts pair again once. Refused if an identity exists.
    pub fn restore_identity(&mut self, backup: &[u8], now_secs: u64) -> CoreResult<()> {
        if self.identity.is_some() {
            return Err(CoreError::NotAllowed);
        }
        let identity = Identity::from_backup(backup)?;
        self.persist_identity_with(&identity, now_secs)?;
        self.identity = Some(identity);
        Ok(())
    }

    fn persist_identity_with(&mut self, identity: &Identity, now_secs: u64) -> CoreResult<()> {
        let (account, addressing) = identity.to_pickle(&self.pickle_key);
        self.conv
            .save_identity(&account, &addressing, secs_i64(now_secs))?;
        Ok(())
    }

    fn persist_identity(&mut self, now_secs: u64) -> CoreResult<()> {
        let identity = self.identity.as_ref().ok_or(CoreError::NoIdentity)?;
        let (account, addressing) = identity.to_pickle(&self.pickle_key);
        self.conv
            .save_identity(&account, &addressing, secs_i64(now_secs))?;
        Ok(())
    }

    /// Creates the QR payload to show. Keep showing the same card until pairing completes.
    pub fn pairing_card(&mut self, display_name: &str, now_secs: u64) -> CoreResult<Vec<u8>> {
        let identity = self.identity.as_mut().ok_or(CoreError::NoIdentity)?;
        let card = identity.create_card(display_name, ONE_TIME_KEYS_PER_CARD)?;
        let bytes = card.encode()?;
        self.pending_card = Some(card);
        self.persist_identity(now_secs)?;
        Ok(bytes)
    }

    /// Completes pairing with a scanned card. Both sides must have scanned each other.
    pub fn complete_pairing(
        &mut self,
        their_card: &[u8],
        now_secs: u64,
    ) -> CoreResult<ContactInfo> {
        let identity = self.identity.as_ref().ok_or(CoreError::NoIdentity)?;
        let mine = self
            .pending_card
            .as_ref()
            .ok_or(CoreError::NoPairingInProgress)?;
        let theirs = PairingCard::decode(their_card)?;
        if theirs.olm_fingerprint_key == identity.fingerprint_key() {
            return Err(CoreError::OwnPairingCode);
        }
        let existing = self.conv.contacts()?.into_iter().find(|c| {
            PairingCard::decode(&c.card)
                .is_ok_and(|k| k.olm_fingerprint_key == theirs.olm_fingerprint_key)
        });
        let keys = identity.pair(mine, &theirs)?;
        let my_fingerprint = identity.fingerprint_key();
        if let Some(c) = existing {
            // Re-pairing in person with the same identity key: fresh pairwise
            // keys, tags and one-time keys; history and verification are kept.
            self.relay.put_pair_keys(&c.contact_ref, &keys.to_bytes())?;
            self.refresh_tags_for(&c.contact_ref, &keys, now_secs)?;
            self.conv.replace_contact_card(&c.contact_id, their_card)?;
            self.pending_card = None;
            return Ok(ContactInfo {
                contact_id: c.contact_id,
                display_name: c.display_name,
                fingerprint: theirs.olm_fingerprint_key,
                id_code: id_code(&theirs.olm_fingerprint_key),
                verified: c.verified,
                blocked: c.blocked,
                safety_number: safety_number(&my_fingerprint, &theirs.olm_fingerprint_key),
                last_seen_secs: None,
                introduced_by: None,
            });
        }
        let contact = Contact {
            contact_id: random_array()?,
            contact_ref: random_array()?,
            display_name: theirs.display_name.clone(),
            card: their_card.to_vec(),
            verified: false,
            paired_at: secs_i64(now_secs),
            next_otk: 0,
            blocked: false,
        };
        self.relay
            .put_pair_keys(&contact.contact_ref, &keys.to_bytes())?;
        self.refresh_tags_for(&contact.contact_ref, &keys, now_secs)?;
        self.conv.add_contact(&contact)?;
        self.pending_card = None;
        Ok(ContactInfo {
            contact_id: contact.contact_id,
            display_name: contact.display_name,
            fingerprint: theirs.olm_fingerprint_key,
            id_code: id_code(&theirs.olm_fingerprint_key),
            verified: false,
            blocked: false,
            safety_number: safety_number(&my_fingerprint, &theirs.olm_fingerprint_key),
            last_seen_secs: None,
            introduced_by: None,
        })
    }

    /// [`Self::pairing_card`] as QR text (`VL1:` + Base45, Doc 21 §6).
    pub fn pairing_code(&mut self, display_name: &str, now_secs: u64) -> CoreResult<String> {
        Ok(encode_pairing_code(
            &self.pairing_card(display_name, now_secs)?,
        ))
    }

    /// [`Self::complete_pairing`] from scanned QR text.
    pub fn complete_pairing_code(&mut self, code: &str, now_secs: u64) -> CoreResult<ContactInfo> {
        let card = decode_pairing_code(code.trim())?;
        self.complete_pairing(&card, now_secs)
    }

    pub fn contacts(&self) -> CoreResult<Vec<ContactInfo>> {
        let me = self
            .identity
            .as_ref()
            .ok_or(CoreError::NoIdentity)?
            .fingerprint_key();
        let seen = self.conv.last_seen_all()?;
        self.conv
            .contacts()?
            .into_iter()
            .map(|c| {
                let card = PairingCard::decode(&c.card)?;
                Ok(ContactInfo {
                    contact_id: c.contact_id,
                    display_name: c.display_name,
                    fingerprint: card.olm_fingerprint_key,
                    id_code: id_code(&card.olm_fingerprint_key),
                    verified: c.verified,
                    blocked: c.blocked,
                    safety_number: safety_number(&me, &card.olm_fingerprint_key),
                    last_seen_secs: seen.get(&c.contact_id).copied(),
                    introduced_by: self.introduced_by(&card.olm_fingerprint_key)?,
                })
            })
            .collect()
    }

    pub fn set_verified(&mut self, contact_id: &[u8; 16], verified: bool) -> CoreResult<()> {
        Ok(self.conv.set_verified(contact_id, verified)?)
    }

    fn contact(&self, contact_id: &[u8; 16]) -> CoreResult<Contact> {
        self.conv
            .contacts()?
            .into_iter()
            .find(|c| &c.contact_id == contact_id)
            .ok_or(CoreError::UnknownContact)
    }

    fn contact_by_ref(&self, contact_ref: &[u8; 16]) -> CoreResult<Contact> {
        self.conv
            .contacts()?
            .into_iter()
            .find(|c| &c.contact_ref == contact_ref)
            .ok_or(CoreError::UnknownContact)
    }

    fn pair_keys(&self, contact_ref: &[u8; 16]) -> CoreResult<PairKeys> {
        Ok(PairKeys::from_bytes(
            &self
                .relay
                .pair_keys(contact_ref)?
                .ok_or(CoreError::UnknownContact)?,
        ))
    }

    /// Inbound tags for every epoch an unexpired envelope could have been created in.
    fn refresh_tags_for(
        &mut self,
        contact_ref: &[u8; 16],
        keys: &PairKeys,
        now_secs: u64,
    ) -> CoreResult<()> {
        let window = u64::from(self.policy.max_lifetime_min + self.policy.skew_min) * 60;
        let first = epoch_of(now_secs.saturating_sub(window));
        let last = epoch_of(now_secs + EPOCH_SECONDS);
        let tags: Vec<(u32, [u8; 16])> = (first..=last).map(|e| (e, keys.tag_in(e))).collect();
        Ok(self.relay.set_tags(contact_ref, &tags)?)
    }

    fn load_session(&self, contact_id: &[u8; 16]) -> CoreResult<Option<Session>> {
        match self.conv.sessions(contact_id)?.into_iter().next() {
            Some((_, pickle)) => {
                let p = SessionPickle::from_encrypted(&pickle, &self.pickle_key)
                    .map_err(|_| CryptoError::Pickle)?;
                Ok(Some(Session::from_pickle(p)))
            }
            None => Ok(None),
        }
    }

    fn save_session(
        &mut self,
        contact_id: &[u8; 16],
        session: &Session,
        now_secs: u64,
    ) -> CoreResult<()> {
        let pickle = session.pickle().encrypt(&self.pickle_key);
        Ok(self.conv.save_session(
            contact_id,
            &session.session_id(),
            &pickle,
            secs_i64(now_secs),
        )?)
    }

    /// Encrypts a content event with Olm, wraps it and queues it for routing.
    fn send_content(
        &mut self,
        contact: &Contact,
        content: &Content,
        now_secs: u64,
    ) -> CoreResult<Outgoing> {
        self.send_content_bytes(contact, &content.encode()?, now_secs)
    }

    /// Sends pre-encoded CONTENT bytes without storing a message row. Exists so
    /// tests can play a newer sender using a content type this build lacks.
    #[doc(hidden)]
    pub fn send_raw_content(
        &mut self,
        contact_id: &[u8; 16],
        encoded: &[u8],
        now_secs: u64,
    ) -> CoreResult<[u8; 16]> {
        let contact = self.contact(contact_id)?;
        Ok(self
            .send_content_bytes(&contact, encoded, now_secs)?
            .message_id)
    }

    fn send_content_bytes(
        &mut self,
        contact: &Contact,
        encoded: &[u8],
        now_secs: u64,
    ) -> CoreResult<Outgoing> {
        self.send_content_opts(contact, encoded, now_secs, None, None)
    }

    /// `budget` overrides the initial copy budget; `message_id` keeps a given
    /// id (resend).
    fn send_content_opts(
        &mut self,
        contact: &Contact,
        encoded: &[u8],
        now_secs: u64,
        budget: Option<u8>,
        message_id: Option<[u8; 16]>,
    ) -> CoreResult<Outgoing> {
        if contact.blocked {
            return Err(CoreError::ContactBlocked);
        }
        let keys = self.pair_keys(&contact.contact_ref)?;
        let mut session = match self.load_session(&contact.contact_id)? {
            Some(s) => s,
            None => {
                let card = PairingCard::decode(&contact.card)?;
                let index = usize::try_from(contact.next_otk).unwrap_or(usize::MAX);
                if index >= card.one_time_keys.len() {
                    return Err(CoreError::OutOfOneTimeKeys);
                }
                self.identity
                    .as_ref()
                    .ok_or(CoreError::NoIdentity)?
                    .outbound_session(&card, index)?
            }
        };
        let params = SendParams {
            now_secs,
            sent_at_ms: self.tick(now_secs)?,
            lifetime_min: self.policy.default_lifetime_min,
            copy_budget: budget.unwrap_or_else(|| self.policy.initial_budget()),
        };
        let out = match message_id {
            Some(id) => build_content_bytes_with_id(&keys, &mut session, encoded, params, id)?,
            None => build_content_bytes(&keys, &mut session, encoded, params)?,
        };
        self.save_session(&contact.contact_id, &session, now_secs)?;
        self.relay.insert(Stored {
            envelope: out.envelope.clone(),
            own: true,
            stored_at_min: now_min(now_secs),
            retried: false,
        })?;
        Ok(out)
    }

    /// Encrypts, queues and persists a text message. Returns its message id.
    pub fn send_text(
        &mut self,
        contact_id: &[u8; 16],
        text: &str,
        now_secs: u64,
    ) -> CoreResult<[u8; 16]> {
        self.send_message(contact_id, text, SendOptions::default(), now_secs)
    }

    /// Sends text, optionally as a reply (only the target id is sent, never its
    /// text) or marked forwarded. Carries the conversation's disappearing timer.
    pub fn send_message(
        &mut self,
        contact_id: &[u8; 16],
        text: &str,
        opts: SendOptions,
        now_secs: u64,
    ) -> CoreResult<[u8; 16]> {
        if text.len() > MAX_TEXT_BYTES {
            return Err(CoreError::TextTooLong);
        }
        let contact = self.contact(contact_id)?;
        let timer_secs = self.conv.timer(contact_id)?;
        let content = Content::Text {
            reply_to: opts.reply_to,
            forwarded: opts.forwarded,
            timer_secs,
            text: text.to_owned(),
        };
        let out = self.send_content(&contact, &content, now_secs)?;
        let header = &out.envelope.header;
        self.conv.insert_message(&MessageRow {
            message_id: out.message_id,
            contact_id: *contact_id,
            outgoing: true,
            body: text.to_owned(),
            created_at: secs_i64(now_secs),
            state: MessageState::Queued,
            envelope_id: Some(header.envelope_id),
            ack_secret: Some(out.ack_secret),
            expiry_min: Some(header.expiry_min),
            sent_at_ms: i64::try_from(out.sent_at_ms).unwrap_or(i64::MAX),
            reply_to: opts.reply_to,
            forwarded: opts.forwarded,
            timer_secs,
            expire_at: (timer_secs > 0).then(|| secs_i64(now_secs) + i64::from(timer_secs)),
            ..Default::default()
        })?;
        self.conv.save_draft(contact_id, "", secs_i64(now_secs))?;
        Ok(out.message_id)
    }

    fn message_in(&self, contact_id: &[u8; 16], message_id: &[u8; 16]) -> CoreResult<MessageRow> {
        self.conv
            .message(message_id)?
            .filter(|m| &m.contact_id == contact_id)
            .ok_or(CoreError::UnknownMessage)
    }

    fn own_event(
        contact_id: &[u8; 16],
        target: &[u8; 16],
        out: &Outgoing,
        now_secs: u64,
    ) -> EventMeta {
        EventMeta {
            target: *target,
            contact_id: *contact_id,
            from_me: true,
            sent_at_ms: i64::try_from(out.sent_at_ms).unwrap_or(i64::MAX),
            received_at: secs_i64(now_secs),
        }
    }

    /// Hybrid logical clock: a timestamp never below physical time and always
    /// above every timestamp this node issued or accepted before.
    fn tick(&mut self, now_secs: u64) -> CoreResult<u64> {
        let next = ms_i64(now_secs).max(self.conv.hlc()?.saturating_add(1));
        self.conv.set_hlc(next)?;
        Ok(u64::try_from(next).unwrap_or(0))
    }

    fn clock_now(&self, now_secs: u64) -> CoreResult<i64> {
        Ok(ms_i64(now_secs).max(self.conv.hlc()?))
    }

    fn observe_clock(&mut self, remote_ms: i64, now_secs: u64) -> CoreResult<()> {
        if remote_ms <= ms_i64(now_secs).saturating_add(MAX_CLOCK_DRIFT_MS) {
            self.conv.set_hlc(remote_ms)?;
        }
        Ok(())
    }

    /// Sets (1..=8, see [`REACTIONS`]) or removes (0) my reaction to a message.
    pub fn react(
        &mut self,
        contact_id: &[u8; 16],
        target: &[u8; 16],
        reaction: u8,
        now_secs: u64,
    ) -> CoreResult<()> {
        if usize::from(reaction) > REACTIONS.len() {
            return Err(CoreError::InvalidReaction);
        }
        let m = self.message_in(contact_id, target)?;
        if m.deleted {
            return Err(CoreError::NotAllowed);
        }
        let contact = self.contact(contact_id)?;
        let out = self.send_content(
            &contact,
            &Content::Reaction {
                target: *target,
                reaction,
            },
            now_secs,
        )?;
        self.conv.upsert_reaction(
            &Self::own_event(contact_id, target, &out, now_secs),
            reaction,
            &out.message_id,
        )?;
        Ok(())
    }

    /// Edits one of my messages: within 24 h of sending, at most 10 times.
    pub fn edit_message(
        &mut self,
        contact_id: &[u8; 16],
        target: &[u8; 16],
        text: &str,
        now_secs: u64,
    ) -> CoreResult<()> {
        if text.len() > MAX_TEXT_BYTES {
            return Err(CoreError::TextTooLong);
        }
        let m = self.message_in(contact_id, target)?;
        let seq = self.conv.edit_seq(target, contact_id, true)? + 1;
        if !m.outgoing
            || m.deleted
            || text.is_empty()
            || seq > MAX_EDIT_SEQ
            || self.clock_now(now_secs)? - m.sent_at_ms > EDIT_WINDOW_MS
        {
            return Err(CoreError::NotAllowed);
        }
        let contact = self.contact(contact_id)?;
        let out = self.send_content(
            &contact,
            &Content::Edit {
                target: *target,
                seq,
                text: text.to_owned(),
            },
            now_secs,
        )?;
        self.conv.upsert_edit(
            &Self::own_event(contact_id, target, &out, now_secs),
            seq,
            text,
        )?;
        Ok(())
    }

    /// Asks the contact's device to delete one of my messages (within 72 h) and
    /// replaces it locally with a placeholder. The request cannot recall copies
    /// the recipient has already read, copied or captured.
    pub fn delete_for_everyone(
        &mut self,
        contact_id: &[u8; 16],
        target: &[u8; 16],
        now_secs: u64,
    ) -> CoreResult<()> {
        let m = self.message_in(contact_id, target)?;
        if !m.outgoing || m.deleted || self.clock_now(now_secs)? - m.sent_at_ms > DELETE_WINDOW_MS {
            return Err(CoreError::NotAllowed);
        }
        let contact = self.contact(contact_id)?;
        let out = self.send_content(&contact, &Content::Delete { target: *target }, now_secs)?;
        self.conv
            .add_tombstone(&Self::own_event(contact_id, target, &out, now_secs))?;
        Ok(())
    }

    /// Deletes a message from this device only.
    pub fn delete_for_me(&mut self, message_id: &[u8; 16], now_secs: u64) -> CoreResult<()> {
        Ok(self.conv.delete_message(message_id, secs_i64(now_secs))?)
    }

    /// Sets the conversation's disappearing-message timer (0 = off) and tells the contact.
    pub fn set_disappearing_timer(
        &mut self,
        contact_id: &[u8; 16],
        timer_secs: u32,
        now_secs: u64,
    ) -> CoreResult<()> {
        if timer_secs != 0 && !(MIN_TIMER_SECS..=MAX_TIMER_SECS).contains(&timer_secs) {
            return Err(CoreError::InvalidTimer);
        }
        let contact = self.contact(contact_id)?;
        let out = self.send_content(&contact, &Content::Timer { timer_secs }, now_secs)?;
        self.conv.set_timer(
            contact_id,
            timer_secs,
            i64::try_from(out.sent_at_ms).unwrap_or(i64::MAX),
        )?;
        Ok(())
    }

    pub fn disappearing_timer(&self, contact_id: &[u8; 16]) -> CoreResult<u32> {
        Ok(self.conv.timer(contact_id)?)
    }

    /// Marks received messages read and starts their disappearing timers.
    /// Read receipts are not sent (feature off by default).
    pub fn mark_read(&mut self, contact_id: &[u8; 16], now_secs: u64) -> CoreResult<u32> {
        let n = self.conv.mark_read(contact_id, secs_i64(now_secs))?;
        Ok(u32::try_from(n).unwrap_or(u32::MAX))
    }

    pub fn message(&self, message_id: &[u8; 16]) -> CoreResult<Option<MessageRow>> {
        Ok(self.conv.message(message_id)?)
    }

    pub fn reactions(&self, message_id: &[u8; 16]) -> CoreResult<Vec<Reaction>> {
        Ok(self.conv.reactions(message_id)?)
    }

    pub fn search(
        &self,
        query: &str,
        contact_id: Option<&[u8; 16]>,
        limit: u32,
    ) -> CoreResult<Vec<MessageRow>> {
        Ok(self.conv.search(query, contact_id, limit)?)
    }

    pub fn save_draft(
        &mut self,
        contact_id: &[u8; 16],
        text: &str,
        now_secs: u64,
    ) -> CoreResult<()> {
        if text.len() > MAX_TEXT_BYTES {
            return Err(CoreError::TextTooLong);
        }
        self.contact(contact_id)?;
        Ok(self.conv.save_draft(contact_id, text, secs_i64(now_secs))?)
    }

    pub fn draft(&self, contact_id: &[u8; 16]) -> CoreResult<Option<String>> {
        Ok(self.conv.draft(contact_id)?)
    }

    /// Blocking stops requesting the contact's envelopes (their inbound tags are
    /// withdrawn), drops any that still arrive without a receipt, and refuses to
    /// send to them. Envelopes for other people are still carried.
    pub fn set_blocked(
        &mut self,
        contact_id: &[u8; 16],
        blocked: bool,
        now_secs: u64,
    ) -> CoreResult<()> {
        let contact = self.contact(contact_id)?;
        self.conv.set_blocked(contact_id, blocked)?;
        if blocked {
            self.relay.set_tags(&contact.contact_ref, &[])?;
        } else {
            let keys = self.pair_keys(&contact.contact_ref)?;
            self.refresh_tags_for(&contact.contact_ref, &keys, now_secs)?;
        }
        Ok(())
    }

    pub fn set_pinned(&mut self, message_id: &[u8; 16], pinned: bool) -> CoreResult<bool> {
        Ok(self.conv.set_pinned(message_id, pinned)?)
    }

    pub fn set_starred(&mut self, message_id: &[u8; 16], starred: bool) -> CoreResult<bool> {
        Ok(self.conv.set_starred(message_id, starred)?)
    }

    /// Pinned (`pinned = true`) or starred messages, newest first.
    pub fn flagged(
        &self,
        contact_id: Option<&[u8; 16]>,
        pinned: bool,
    ) -> CoreResult<Vec<MessageRow>> {
        Ok(self.conv.flagged(contact_id, pinned)?)
    }

    pub fn conversation(&self, contact_id: &[u8; 16], limit: u32) -> CoreResult<Vec<MessageRow>> {
        Ok(self.conv.conversation(contact_id, limit)?)
    }

    /// This device's 32-character ID code (derived from the identity key).
    pub fn my_id_code(&self) -> CoreResult<String> {
        Ok(id_code(&self.my_fingerprint()?))
    }

    /// Renames a contact on this phone only (1..=32 bytes).
    pub fn rename_contact(&mut self, contact_id: &[u8; 16], name: &str) -> CoreResult<()> {
        let name = name.trim();
        if name.is_empty() || name.len() > vl_proto::MAX_NAME_LEN {
            return Err(ProtoError::InvalidField("name").into());
        }
        if !self.conv.rename_contact(contact_id, name)? {
            return Err(CoreError::UnknownContact);
        }
        Ok(())
    }

    /// Deletes a contact, the whole conversation with them and their relay-side
    /// keys and tags. Their future envelopes are no longer recognised as mine
    /// (they may still be carried for others). Group rosters are unchanged.
    pub fn delete_contact(&mut self, contact_id: &[u8; 16]) -> CoreResult<()> {
        let contact_ref = self
            .conv
            .delete_contact(contact_id)?
            .ok_or(CoreError::UnknownContact)?;
        self.nearby.remove(contact_id);
        Ok(self.relay.remove_contact(&contact_ref)?)
    }

    /// Re-encrypts an expired or unconfirmed message under the same message id
    /// (the recipient deduplicates) and queues it again.
    pub fn resend(&mut self, message_id: &[u8; 16], now_secs: u64) -> CoreResult<()> {
        let m = self
            .conv
            .message(message_id)?
            .ok_or(CoreError::UnknownMessage)?;
        if !m.outgoing
            || m.deleted
            || !matches!(
                m.state,
                MessageState::Expired | MessageState::DeliveryUnknown
            )
            || self.conv.attachment(message_id)?.is_some()
        {
            return Err(CoreError::CannotResend);
        }
        let contact = self.contact(&m.contact_id)?;
        let content = Content::Text {
            reply_to: m.reply_to,
            forwarded: m.forwarded,
            timer_secs: m.timer_secs,
            text: m.body.clone(),
        };
        let out = self.send_content_opts(
            &contact,
            &content.encode()?,
            now_secs,
            None,
            Some(*message_id),
        )?;
        let h = &out.envelope.header;
        self.conv.set_message_envelope(
            message_id,
            &h.envelope_id,
            &out.ack_secret,
            h.expiry_min,
        )?;
        self.conv
            .transition(message_id, MessageState::Queued, secs_i64(now_secs))?;
        Ok(())
    }

    /// Bytes this phone carries for others (relay quota), 1 MiB ..= 100 MiB.
    pub fn set_carry_cap(&mut self, bytes: u64) {
        let b = usize::try_from(bytes).unwrap_or(usize::MAX);
        self.policy.carry_cap_bytes = b.clamp(1 << 20, 100 << 20);
    }

    pub fn carry_cap(&self) -> u64 {
        u64::try_from(self.policy.carry_cap_bytes).unwrap_or(u64::MAX)
    }

    /// Seconds a phone waits after an exchange before syncing with the same
    /// nearby phone again (ADR-032). User-set, 1 ..= 45, default 5.
    pub fn resync_secs(&self) -> CoreResult<u32> {
        let stored = self.conv.setting(RESYNC_SETTING)?;
        Ok(stored
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(RESYNC_DEFAULT_SECS)
            .clamp(RESYNC_MIN_SECS, RESYNC_MAX_SECS))
    }

    /// Stores the re-sync interval, clamped to 1 ..= 45 s; returns what was stored.
    pub fn set_resync_secs(&mut self, secs: u32) -> CoreResult<u32> {
        let v = secs.clamp(RESYNC_MIN_SECS, RESYNC_MAX_SECS);
        self.conv
            .set_setting(RESYNC_SETTING, Some(&v.to_string()))?;
        Ok(v)
    }

    /// Sends an image (≤ 16 KiB, already re-encoded by the host) or a file
    /// (≤ 64 KiB) as an attachment message (ADR-026). The announcement is an
    /// Olm content event carrying a one-time key; the chunks are sealed with
    /// that key and sent as `AttachmentChunk` envelopes (budget 2, to limit
    /// relay load). Each chunk's delivery is proven separately.
    #[allow(clippy::too_many_arguments)]
    pub fn send_attachment(
        &mut self,
        contact_id: &[u8; 16],
        kind: u8,
        name: &str,
        data: &[u8],
        caption: &str,
        reply_to: Option<[u8; 16]>,
        now_secs: u64,
    ) -> CoreResult<[u8; 16]> {
        let total_len = u32::try_from(data.len()).map_err(|_| CoreError::InvalidAttachment)?;
        let attachment_id: [u8; 16] = random_array()?;
        let key: [u8; 32] = random_array()?;
        let content = Content::Attachment {
            attachment_id,
            kind,
            total_len,
            sha256: sha256(data),
            key,
            reply_to,
            name: name.to_owned(),
            caption: caption.to_owned(),
        };
        let encoded = content.encode().map_err(|_| CoreError::InvalidAttachment)?;
        let contact = self.contact(contact_id)?;
        let out = self.send_content_bytes(&contact, &encoded, now_secs)?;
        let header = &out.envelope.header;
        self.conv.insert_message(&MessageRow {
            message_id: out.message_id,
            contact_id: *contact_id,
            outgoing: true,
            body: caption.to_owned(),
            created_at: secs_i64(now_secs),
            state: MessageState::Queued,
            envelope_id: Some(header.envelope_id),
            ack_secret: Some(out.ack_secret),
            expiry_min: Some(header.expiry_min),
            sent_at_ms: i64::try_from(out.sent_at_ms).unwrap_or(i64::MAX),
            reply_to,
            ..Default::default()
        })?;
        self.conv.put_attachment(
            &AttachmentRow {
                message_id: out.message_id,
                attachment_id,
                contact_id: *contact_id,
                outgoing: true,
                kind,
                name: name.to_owned(),
                total_len,
                sha256: sha256(data),
                key,
                received_chunks: 0,
                complete: true,
            },
            Some(data),
        )?;
        let keys = self.pair_keys(&contact.contact_ref)?;
        for (index, piece) in data.chunks(CHUNK_BYTES).enumerate() {
            let index = u16::try_from(index).map_err(|_| CoreError::InvalidAttachment)?;
            let chunk = ChunkPayload {
                attachment_id,
                index,
                ciphertext: seal_chunk(&key, &attachment_id, index, piece)?,
            };
            let params = SendParams {
                now_secs,
                sent_at_ms: self.tick(now_secs)?,
                lifetime_min: self.policy.default_lifetime_min,
                copy_budget: 2,
            };
            let c = build_attachment_chunk(&keys, &chunk, params)?;
            self.relay.insert(Stored {
                envelope: c.envelope.clone(),
                own: true,
                stored_at_min: now_min(now_secs),
                retried: false,
            })?;
            self.conv.add_group_copy(
                &c.message_id,
                &attachment_id,
                contact_id,
                &c.envelope.header.envelope_id,
                &c.ack_secret,
                c.envelope.header.expiry_min,
            )?;
        }
        Ok(out.message_id)
    }

    /// Attachment metadata for a message, if it has one.
    pub fn attachment(&self, message_id: &[u8; 16]) -> CoreResult<Option<AttachmentRow>> {
        Ok(self.conv.attachment(message_id)?)
    }

    /// The bytes of a complete attachment.
    pub fn attachment_data(&self, message_id: &[u8; 16]) -> CoreResult<Option<Vec<u8>>> {
        Ok(self.conv.attachment_data(message_id)?)
    }

    /// For my attachments: chunks whose delivery was proven, and chunks sent.
    pub fn attachment_delivery(&self, message_id: &[u8; 16]) -> CoreResult<(u32, u32)> {
        match self.conv.attachment(message_id)? {
            Some(a) if a.outgoing => Ok(self.conv.group_delivery(&a.attachment_id)?),
            _ => Ok((0, 0)),
        }
    }

    /// Stores a received chunk and completes the attachment when all chunks
    /// are present and the SHA-256 matches. A mismatch discards the chunks.
    fn on_chunk(
        &mut self,
        contact_id: &[u8; 16],
        attachment_id: &[u8; 16],
        index: u16,
        data: &[u8],
        now_secs: u64,
    ) -> CoreResult<()> {
        self.conv
            .add_chunk(contact_id, attachment_id, index, data, secs_i64(now_secs))?;
        self.try_complete_attachment(contact_id, attachment_id)
    }

    fn try_complete_attachment(
        &mut self,
        contact_id: &[u8; 16],
        attachment_id: &[u8; 16],
    ) -> CoreResult<()> {
        let Some(a) = self.conv.attachment_by_id(contact_id, attachment_id)? else {
            return Ok(());
        };
        if a.complete {
            return Ok(());
        }
        let count = chunk_count_for(a.total_len);
        let Some(sealed) = self
            .conv
            .assembled_attachment(contact_id, attachment_id, count)?
        else {
            return Ok(());
        };
        let mut bytes = Vec::with_capacity(usize::try_from(a.total_len).unwrap_or(0));
        let mut ok = true;
        for (index, piece) in sealed.iter().enumerate() {
            let opened = u16::try_from(index)
                .ok()
                .and_then(|i| open_chunk(&a.key, attachment_id, i, piece).ok());
            match opened {
                Some(p) => bytes.extend_from_slice(&p),
                None => {
                    ok = false;
                    break;
                }
            }
        }
        let ok = ok
            && u32::try_from(bytes.len()).ok() == Some(a.total_len)
            && sha256(&bytes) == a.sha256;
        self.conv
            .finish_attachment(contact_id, attachment_id, ok.then_some(&bytes[..]))?;
        Ok(())
    }

    /// Every contact with its newest message, most recent activity first;
    /// contacts without messages come last.
    pub fn conversation_summaries(&self) -> CoreResult<Vec<ConversationSummary>> {
        let mut out = self
            .contacts()?
            .into_iter()
            .map(|contact| {
                let last = self
                    .conv
                    .conversation(&contact.contact_id, 1)?
                    .into_iter()
                    .next();
                let unread = self.conv.unread_count(&contact.contact_id)?;
                Ok(ConversationSummary {
                    contact,
                    last,
                    unread,
                })
            })
            .collect::<CoreResult<Vec<_>>>()?;
        out.sort_by_key(|s| std::cmp::Reverse(s.last.as_ref().map(|m| m.created_at)));
        Ok(out)
    }

    /// My outgoing messages without delivery proof yet, newest first.
    pub fn pending_outgoing(&self) -> CoreResult<Vec<MessageRow>> {
        Ok(self.conv.pending_outgoing()?)
    }

    pub fn message_events(&self, message_id: &[u8; 16]) -> CoreResult<Vec<(i64, MessageState)>> {
        Ok(self.conv.message_events(message_id)?)
    }

    pub fn setting(&self, key: &str) -> CoreResult<Option<String>> {
        Ok(self.conv.setting(key)?)
    }

    pub fn set_setting(&mut self, key: &str, value: Option<&str>) -> CoreResult<()> {
        Ok(self.conv.set_setting(key, value)?)
    }

    pub fn stats(&self) -> CoreResult<NodeStats> {
        let (own_envelopes, carried_envelopes, carried_bytes) = self.relay.usage()?;
        Ok(NodeStats {
            open_links: u32::try_from(self.links.len()).unwrap_or(u32::MAX),
            own_envelopes,
            carried_envelopes,
            carried_bytes,
        })
    }

    /// Expiry, re-spray and tag rotation. Call on every wake and foreground entry.
    pub fn maintain(&mut self, now_secs: u64) -> CoreResult<()> {
        let events = maintain(&mut self.relay, &self.policy, now_min(now_secs))?;
        self.apply_events(events, now_secs)?;
        self.conv.purge_expired(secs_i64(now_secs))?;
        for c in self.conv.contacts()?.into_iter().filter(|c| !c.blocked) {
            let keys = self.pair_keys(&c.contact_ref)?;
            self.refresh_tags_for(&c.contact_ref, &keys, now_secs)?;
        }
        Ok(())
    }

    /// Turns routing events into message-state changes and local actions.
    /// Returns true if new local state should be offered to the current peer.
    fn apply_events(&mut self, events: Vec<Event>, now_secs: u64) -> CoreResult<bool> {
        let at = secs_i64(now_secs);
        let mut changed = false;
        for event in events {
            match event {
                Event::Inbound(env) => changed |= self.on_inbound(&env, now_secs)?,
                Event::OwnAccepted(eid) => {
                    if let Some(mid) = self.conv.message_by_envelope(&eid)? {
                        self.conv
                            .transition(&mid, MessageState::AcceptedByRelay, at)?;
                    }
                }
                Event::OwnDelivered { envelope_id, .. } => {
                    if let Some(mid) = self.conv.message_by_envelope(&envelope_id)? {
                        self.conv.transition(&mid, MessageState::Acknowledged, at)?;
                    } else {
                        self.conv.mark_group_copy_delivered(&envelope_id, None)?;
                    }
                }
                Event::OwnExpired(eid) => {
                    if let Some(mid) = self.conv.message_by_envelope(&eid)?
                        && !self.conv.transition(&mid, MessageState::Expired, at)?
                    {
                        self.conv
                            .transition(&mid, MessageState::DeliveryUnknown, at)?;
                    }
                }
                Event::PeerIncompatible | Event::Closed => {}
            }
        }
        Ok(changed)
    }

    fn on_inbound(&mut self, env: &Envelope, now_secs: u64) -> CoreResult<bool> {
        let Some(contact_ref) = self.relay.contact_for_tag(&env.header.recipient_tag)? else {
            return Ok(false);
        };
        let contact = self.contact_by_ref(&contact_ref)?;
        if contact.blocked {
            return Ok(false);
        }
        let keys = self.pair_keys(&contact_ref)?;
        let Ok(opened) = open_from(&keys, env) else {
            // Forged or corrupted: don't let it occupy the envelope id, or a
            // malicious relay could block the genuine copy as a "duplicate".
            self.relay.forget_seen(&env.header.envelope_id)?;
            return Ok(false);
        };
        let params = SendParams {
            now_secs,
            sent_at_ms: self.tick(now_secs)?,
            lifetime_min: self.policy.default_lifetime_min,
            copy_budget: self.policy.initial_budget(),
        };
        match opened.inner.kind {
            InnerKind::Text | InnerKind::Content | InnerKind::AttachmentChunk => {
                let applied = if opened.inner.kind == InnerKind::AttachmentChunk {
                    self.receive_chunk(&contact, &opened, now_secs)
                } else {
                    self.receive_olm(&contact, &opened, now_secs)
                };
                let receipt_kind = match applied {
                    Ok(()) => InnerKind::ReceiptDelivered,
                    Err(_) => InnerKind::ReceiptUndecryptable,
                };
                if let Some(secret) = opened.vaccine {
                    let item = VaccineItem {
                        ack_secret: secret,
                        expiry_min: env.header.expiry_min,
                    };
                    self.relay.add_vaccine(item, env.header.ack_commitment)?;
                }
                let receipt =
                    build_receipt(&keys, receipt_kind, &[opened.inner.message_id], params)?;
                self.relay.insert(Stored {
                    envelope: receipt,
                    own: true,
                    stored_at_min: now_min(now_secs),
                    retried: false,
                })?;
                Ok(true)
            }
            InnerKind::ReceiptDelivered | InnerKind::ReceiptUndecryptable => {
                let (kind, receipt) = verify_receipt(&keys, env)?;
                let next = if kind == InnerKind::ReceiptDelivered {
                    MessageState::Acknowledged
                } else {
                    MessageState::Failed
                };
                let mut changed = false;
                for mid in receipt.refs {
                    let Some(m) = self
                        .conv
                        .message(&mid)?
                        .filter(|m| m.outgoing && m.contact_id == contact.contact_id)
                    else {
                        if next == MessageState::Acknowledged
                            && let Some((secret, expiry)) = self
                                .conv
                                .mark_group_copy_delivered(&mid, Some(&contact.contact_id))?
                        {
                            self.relay.add_vaccine(
                                VaccineItem {
                                    ack_secret: secret,
                                    expiry_min: expiry,
                                },
                                vl_crypto::ack_commitment(&secret),
                            )?;
                            changed = true;
                        }
                        continue;
                    };
                    if self.conv.transition(&mid, next, secs_i64(now_secs))?
                        && next == MessageState::Acknowledged
                        && let (Some(secret), Some(expiry)) = (m.ack_secret, m.expiry_min)
                    {
                        self.relay.add_vaccine(
                            VaccineItem {
                                ack_secret: secret,
                                expiry_min: expiry,
                            },
                            vl_crypto::ack_commitment(&secret),
                        )?;
                        changed = true;
                    }
                }
                Ok(changed)
            }
            _ => Ok(false),
        }
    }

    /// Stores a sealed attachment chunk (opened once the announcement and every
    /// chunk are present). A malformed payload counts as undecryptable.
    fn receive_chunk(
        &mut self,
        contact: &Contact,
        opened: &vl_crypto::Opened,
        now_secs: u64,
    ) -> CoreResult<()> {
        let chunk = ChunkPayload::decode(&opened.inner.payload)?;
        self.on_chunk(
            &contact.contact_id,
            &chunk.attachment_id,
            chunk.index,
            &chunk.ciphertext,
            now_secs,
        )
    }

    /// Decrypts an Olm-protected inner message and applies it. Unknown content
    /// types still count as delivered (a newer sender), malformed ones do not.
    fn receive_olm(
        &mut self,
        contact: &Contact,
        opened: &vl_crypto::Opened,
        now_secs: u64,
    ) -> CoreResult<()> {
        let plain = self.decrypt_olm_payload(contact, opened, now_secs)?;
        let content = match opened.inner.kind {
            InnerKind::Text => Content::Text {
                reply_to: None,
                forwarded: false,
                timer_secs: 0,
                text: String::from_utf8(plain)
                    .map_err(|_| CryptoError::Olm("message is not UTF-8".into()))?,
            },
            _ => match Content::decode(&plain) {
                Ok(c) => c,
                Err(ProtoError::UnknownType(_)) => return Ok(()),
                Err(e) => return Err(e.into()),
            },
        };
        let sent_at_ms = i64::try_from(opened.inner.sent_at_ms).unwrap_or(i64::MAX);
        self.observe_clock(sent_at_ms, now_secs)?;
        let ev = |target: [u8; 16]| EventMeta {
            target,
            contact_id: contact.contact_id,
            from_me: false,
            sent_at_ms,
            received_at: secs_i64(now_secs),
        };
        match content {
            Content::Text {
                reply_to,
                forwarded,
                timer_secs,
                text,
            } => {
                if text.len() > MAX_TEXT_BYTES {
                    return Err(CoreError::TextTooLong);
                }
                if opened.inner.kind == InnerKind::Content {
                    self.conv
                        .set_timer(&contact.contact_id, timer_secs, sent_at_ms)?;
                }
                self.conv.insert_message(&MessageRow {
                    message_id: opened.inner.message_id,
                    contact_id: contact.contact_id,
                    outgoing: false,
                    body: text,
                    created_at: secs_i64(now_secs),
                    state: MessageState::Received,
                    sent_at_ms,
                    reply_to,
                    forwarded,
                    timer_secs,
                    ..Default::default()
                })?;
            }
            Content::Reaction { target, reaction } => {
                self.conv
                    .upsert_reaction(&ev(target), reaction, &opened.inner.message_id)?;
            }
            Content::Edit { target, seq, text } => {
                if text.len() > MAX_TEXT_BYTES {
                    return Err(CoreError::TextTooLong);
                }
                self.conv.upsert_edit(&ev(target), seq, &text)?;
            }
            Content::Delete { target } => {
                self.conv.add_tombstone(&ev(target))?;
            }
            Content::Timer { timer_secs } => {
                self.conv
                    .set_timer(&contact.contact_id, timer_secs, sent_at_ms)?;
            }
            Content::GroupState {
                group_id,
                epoch,
                name,
                members,
            } => self.on_group_state(contact, group_id, epoch, name, members, now_secs)?,
            Content::GroupText {
                group_id,
                group_msg_id,
                epoch,
                reply_to,
                text,
            } => {
                if text.len() > MAX_TEXT_BYTES {
                    return Err(CoreError::TextTooLong);
                }
                let p = PendingGroupMessage {
                    contact_id: contact.contact_id,
                    epoch,
                    message: GroupMessageRow {
                        group_msg_id,
                        group_id,
                        author_fp: [0; 32],
                        outgoing: false,
                        body: text,
                        reply_to,
                        created_at: secs_i64(now_secs),
                        sent_at_ms,
                    },
                };
                self.on_group_text(contact, p, now_secs)?;
            }
            Content::GroupLeave { group_id } => self.on_group_leave(contact, group_id, now_secs)?,
            Content::Location {
                lat_e7,
                lon_e7,
                accuracy_m,
                captured_ms,
            } => self.on_location(
                contact,
                opened.inner.message_id,
                sent_at_ms,
                vl_store::LocationRow {
                    lat_e7,
                    lon_e7,
                    accuracy_m,
                    captured_ms,
                },
                now_secs,
            )?,
            Content::Poll {
                group,
                poll_id,
                question,
                options,
                multi,
            } => self.on_poll(
                contact,
                opened.inner.message_id,
                sent_at_ms,
                group,
                poll_id,
                vl_store::PollRow {
                    question,
                    options,
                    multi,
                },
                now_secs,
            )?,
            Content::PollVote {
                group,
                poll_id,
                choices,
            } => {
                self.on_poll_vote(contact, group, poll_id, choices, now_secs)?;
            }
            Content::IntroAsk {
                intro_id,
                other_fp,
                other_name,
            } => self.on_intro_ask(
                contact,
                opened.inner.message_id,
                sent_at_ms,
                intro_id,
                other_fp,
                other_name,
                now_secs,
            )?,
            Content::IntroCard { intro_id, card } => {
                self.on_intro_card(contact, intro_id, card, now_secs)?;
            }
            Content::IntroDeliver { intro_id, card } => {
                self.on_intro_deliver(contact, intro_id, card, now_secs)?;
            }
            Content::Attachment {
                attachment_id,
                kind,
                total_len,
                sha256: digest,
                key,
                reply_to,
                name,
                caption,
            } => {
                self.conv.insert_message(&MessageRow {
                    message_id: opened.inner.message_id,
                    contact_id: contact.contact_id,
                    outgoing: false,
                    body: caption,
                    created_at: secs_i64(now_secs),
                    state: MessageState::Received,
                    sent_at_ms,
                    reply_to,
                    ..Default::default()
                })?;
                let deleted = self
                    .conv
                    .message(&opened.inner.message_id)?
                    .is_none_or(|m| m.deleted);
                if deleted {
                    return Ok(());
                }
                self.conv.put_attachment(
                    &AttachmentRow {
                        message_id: opened.inner.message_id,
                        attachment_id,
                        contact_id: contact.contact_id,
                        outgoing: false,
                        kind,
                        name,
                        total_len,
                        sha256: digest,
                        key,
                        received_chunks: 0,
                        complete: false,
                    },
                    None,
                )?;
                self.try_complete_attachment(&contact.contact_id, &attachment_id)?;
            }
        }
        Ok(())
    }

    fn decrypt_olm_payload(
        &mut self,
        contact: &Contact,
        opened: &vl_crypto::Opened,
        now_secs: u64,
    ) -> CoreResult<Vec<u8>> {
        let payload: OlmPayload = olm_payload(opened)?;
        let card = PairingCard::decode(&contact.card)?;
        let identity = self.identity.as_mut().ok_or(CoreError::NoIdentity)?;
        let existing = self
            .conv
            .sessions(&contact.contact_id)?
            .into_iter()
            .filter_map(|(_, p)| {
                SessionPickle::from_encrypted(&p, &self.pickle_key)
                    .ok()
                    .map(Session::from_pickle)
            });
        let mut result = None;
        for s in existing {
            if let Ok(ok) = decrypt_olm(identity, &card.olm_identity_key, Some(s), &payload) {
                result = Some(ok);
                break;
            }
        }
        let (session, plain) = match result {
            Some(r) => r,
            None if payload.is_pre_key() => {
                decrypt_olm(identity, &card.olm_identity_key, None, &payload)?
            }
            None => return Err(CryptoError::Olm("no matching session".into()).into()),
        };
        self.save_session(&contact.contact_id, &session, now_secs)?;
        self.persist_identity(now_secs)?;
        Ok(plain)
    }

    /// This device's identity (Ed25519) key; group rosters identify members by it.
    pub fn my_fingerprint(&self) -> CoreResult<[u8; 32]> {
        Ok(self
            .identity
            .as_ref()
            .ok_or(CoreError::NoIdentity)?
            .fingerprint_key())
    }

    fn fingerprint_of(contact: &Contact) -> CoreResult<[u8; 32]> {
        Ok(PairingCard::decode(&contact.card)?.olm_fingerprint_key)
    }

    fn contact_by_fingerprint(&self, fp: &[u8; 32]) -> CoreResult<Option<Contact>> {
        for c in self.conv.contacts()? {
            if &Self::fingerprint_of(&c)? == fp {
                return Ok(Some(c));
            }
        }
        Ok(None)
    }

    /// Sends the roster to every listed fingerprint I am paired with (except me).
    fn broadcast_group_state(
        &mut self,
        g: &GroupRow,
        recipients: &[[u8; 32]],
        now_secs: u64,
    ) -> CoreResult<()> {
        let me = self.my_fingerprint()?;
        let content = Content::GroupState {
            group_id: g.group_id,
            epoch: g.epoch,
            name: g.name.clone(),
            members: g
                .members
                .iter()
                .map(|(fp, name)| GroupMember {
                    fingerprint: *fp,
                    name: name.clone(),
                })
                .collect(),
        };
        for fp in recipients.iter().filter(|fp| **fp != me) {
            if let Some(c) = self.contact_by_fingerprint(fp)?.filter(|c| !c.blocked) {
                self.send_content(&c, &content, now_secs)?;
            }
        }
        Ok(())
    }

    fn roster(
        &self,
        me_name: &str,
        contact_ids: &[[u8; 16]],
    ) -> CoreResult<Vec<([u8; 32], String)>> {
        let mut members = vec![(self.my_fingerprint()?, me_name.to_owned())];
        for id in contact_ids {
            let c = self.contact(id)?;
            if c.blocked {
                return Err(CoreError::ContactBlocked);
            }
            let fp = Self::fingerprint_of(&c)?;
            if !members.iter().any(|(m, _)| *m == fp) {
                members.push((fp, c.display_name.clone()));
            }
        }
        if members.len() < 2 || members.len() > MAX_GROUP_MEMBERS {
            return Err(CoreError::InvalidGroupSize);
        }
        Ok(members)
    }

    /// Creates a small group (ADR-024) and sends its roster to every member.
    /// `my_name` is how I appear to members who know me only through the group.
    pub fn create_group(
        &mut self,
        name: &str,
        my_name: &str,
        contact_ids: &[[u8; 16]],
        now_secs: u64,
    ) -> CoreResult<[u8; 16]> {
        let members = self.roster(my_name, contact_ids)?;
        let me = self.my_fingerprint()?;
        let g = GroupRow {
            group_id: random_array()?,
            name: name.to_owned(),
            creator_fp: me,
            epoch: 1,
            left: false,
            members,
        };
        self.conv.apply_group_state(&g, &me, secs_i64(now_secs))?;
        let recipients: Vec<[u8; 32]> = g.members.iter().map(|(fp, _)| *fp).collect();
        self.broadcast_group_state(&g, &recipients, now_secs)?;
        Ok(g.group_id)
    }

    /// Replaces the member list (creator only). Removed members are told too,
    /// so they stop treating the group as theirs.
    pub fn set_group_members(
        &mut self,
        group_id: &[u8; 16],
        my_name: &str,
        contact_ids: &[[u8; 16]],
        now_secs: u64,
    ) -> CoreResult<()> {
        let old = self.conv.group(group_id)?.ok_or(CoreError::UnknownGroup)?;
        let me = self.my_fingerprint()?;
        if old.creator_fp != me {
            return Err(CoreError::NotGroupCreator);
        }
        let g = GroupRow {
            epoch: old.epoch.saturating_add(1),
            members: self.roster(my_name, contact_ids)?,
            ..old.clone()
        };
        self.conv.apply_group_state(&g, &me, secs_i64(now_secs))?;
        let mut recipients: Vec<[u8; 32]> = g.members.iter().map(|(fp, _)| *fp).collect();
        recipients.extend(old.members.iter().map(|(fp, _)| *fp));
        recipients.sort_unstable();
        recipients.dedup();
        self.broadcast_group_state(&g, &recipients, now_secs)
    }

    /// Leaves locally and asks the creator to remove me.
    pub fn leave_group(&mut self, group_id: &[u8; 16], now_secs: u64) -> CoreResult<()> {
        let g = self.conv.group(group_id)?.ok_or(CoreError::UnknownGroup)?;
        self.conv.set_group_left(group_id)?;
        if g.creator_fp != self.my_fingerprint()?
            && let Some(creator) = self.contact_by_fingerprint(&g.creator_fp)?
            && !creator.blocked
        {
            self.send_content(
                &creator,
                &Content::GroupLeave {
                    group_id: *group_id,
                },
                now_secs,
            )?;
        }
        Ok(())
    }

    pub fn groups(&self) -> CoreResult<Vec<GroupRow>> {
        Ok(self.conv.groups()?)
    }

    pub fn group_messages(
        &self,
        group_id: &[u8; 16],
        limit: u32,
    ) -> CoreResult<Vec<GroupMessageRow>> {
        Ok(self.conv.group_messages(group_id, limit)?)
    }

    /// `(delivered, sent)` pairwise copies of one of my group messages. Members I
    /// am not paired with receive no copy and are not counted.
    pub fn group_delivery(&self, group_msg_id: &[u8; 16]) -> CoreResult<(u32, u32)> {
        Ok(self.conv.group_delivery(group_msg_id)?)
    }

    /// Sends one pairwise copy to every member I am paired with (ADR-024).
    pub fn send_group_message(
        &mut self,
        group_id: &[u8; 16],
        text: &str,
        reply_to: Option<[u8; 16]>,
        now_secs: u64,
    ) -> CoreResult<[u8; 16]> {
        if text.len() > MAX_TEXT_BYTES {
            return Err(CoreError::TextTooLong);
        }
        let g = self.conv.group(group_id)?.ok_or(CoreError::UnknownGroup)?;
        let me = self.my_fingerprint()?;
        if g.left || !g.members.iter().any(|(fp, _)| *fp == me) {
            return Err(CoreError::NotAllowed);
        }
        let group_msg_id: [u8; 16] = random_array()?;
        let content = Content::GroupText {
            group_id: *group_id,
            group_msg_id,
            epoch: g.epoch,
            reply_to,
            text: text.to_owned(),
        };
        let mut sent_at_ms = 0;
        for (fp, _) in g.members.iter().filter(|(fp, _)| *fp != me) {
            let Some(c) = self.contact_by_fingerprint(fp)?.filter(|c| !c.blocked) else {
                continue;
            };
            let out = self.send_content(&c, &content, now_secs)?;
            sent_at_ms = i64::try_from(out.sent_at_ms).unwrap_or(i64::MAX);
            self.conv.add_group_copy(
                &out.message_id,
                &group_msg_id,
                &c.contact_id,
                &out.envelope.header.envelope_id,
                &out.ack_secret,
                out.envelope.header.expiry_min,
            )?;
        }
        self.conv.insert_group_message(&GroupMessageRow {
            group_msg_id,
            group_id: *group_id,
            author_fp: me,
            outgoing: true,
            body: text.to_owned(),
            reply_to,
            created_at: secs_i64(now_secs),
            sent_at_ms,
        })?;
        Ok(group_msg_id)
    }

    fn on_group_state(
        &mut self,
        contact: &Contact,
        group_id: [u8; 16],
        epoch: u32,
        name: String,
        members: Vec<GroupMember>,
        now_secs: u64,
    ) -> CoreResult<()> {
        let sender = Self::fingerprint_of(contact)?;
        let me = self.my_fingerprint()?;
        let known = self.conv.group(&group_id)?.is_some();
        let i_am_member = members.iter().any(|m| m.fingerprint == me);
        if !known && (!i_am_member || !members.iter().any(|m| m.fingerprint == sender)) {
            return Ok(());
        }
        let g = GroupRow {
            group_id,
            name,
            creator_fp: sender,
            epoch,
            left: !i_am_member,
            members: members
                .into_iter()
                .map(|m| (m.fingerprint, m.name))
                .collect(),
        };
        if self
            .conv
            .apply_group_state(&g, &sender, secs_i64(now_secs))?
            == GroupApply::Applied
        {
            for p in self.conv.take_group_pending(&group_id)? {
                let from = self.contact(&p.contact_id)?;
                self.on_group_text(&from, p, now_secs)?;
            }
        }
        Ok(())
    }

    /// Accepts a group message only from a current member of a group I am in.
    /// Messages for an unknown group, or claiming a newer epoch than I hold, wait
    /// (bounded) for the roster; messages from non-members are dropped.
    fn on_group_text(
        &mut self,
        contact: &Contact,
        mut p: PendingGroupMessage,
        now_secs: u64,
    ) -> CoreResult<()> {
        let sender = Self::fingerprint_of(contact)?;
        let gid = p.message.group_id;
        match self.conv.group(&gid)? {
            Some(g) if g.left => {}
            Some(g) if p.epoch <= g.epoch => {
                if g.members.iter().any(|(fp, _)| *fp == sender) {
                    p.message.author_fp = sender;
                    self.conv.insert_group_message(&p.message)?;
                }
            }
            _ => {
                self.conv.hold_group_message(&p, secs_i64(now_secs))?;
            }
        }
        Ok(())
    }

    fn on_group_leave(
        &mut self,
        contact: &Contact,
        group_id: [u8; 16],
        now_secs: u64,
    ) -> CoreResult<()> {
        let leaver = Self::fingerprint_of(contact)?;
        let Some(g) = self.conv.group(&group_id)? else {
            return Ok(());
        };
        if g.creator_fp != self.my_fingerprint()? || !g.members.iter().any(|(fp, _)| *fp == leaver)
        {
            return Ok(());
        }
        let next = GroupRow {
            epoch: g.epoch.saturating_add(1),
            members: g
                .members
                .iter()
                .filter(|(fp, _)| *fp != leaver)
                .cloned()
                .collect(),
            ..g.clone()
        };
        let me = next.creator_fp;
        self.conv
            .apply_group_state(&next, &me, secs_i64(now_secs))?;
        let recipients: Vec<[u8; 32]> = g.members.iter().map(|(fp, _)| *fp).collect();
        self.broadcast_group_state(&next, &recipients, now_secs)
    }

    /// Panic wipe. Closes both databases, drops every key and session held in
    /// memory and deletes the database files with their WAL/SHM/journal side
    /// files. Envelopes carried for other people are destroyed too.
    ///
    /// Deleting files on flash storage does not guarantee the bytes are gone, so
    /// the guarantee is cryptographic erasure: the host must delete both database
    /// keys from the Keychain/Keystore **before** calling this.
    pub fn panic_wipe(self) -> CoreResult<()> {
        let paths = self.db_paths.clone();
        drop(self);
        remove_database_files(&paths)
    }

    pub fn relay_store(&self) -> &dyn Store {
        &self.relay
    }
}

/// Deletes database files with their WAL/SHM/journal side files; missing files
/// are fine. Used by [`Node::panic_wipe`], and by hosts when no node can be
/// opened (for example after the keys were lost). Delete the keys first.
pub fn remove_database_files(paths: &[PathBuf]) -> CoreResult<()> {
    let mut first_error = None;
    for db in paths {
        for suffix in ["", "-wal", "-shm", "-journal"] {
            let mut name = db.clone().into_os_string();
            name.push(suffix);
            match std::fs::remove_file(&name) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => {
                    first_error.get_or_insert(e);
                }
            }
        }
    }
    match first_error {
        None => Ok(()),
        Some(e) => Err(CoreError::Wipe(e.kind())),
    }
}
