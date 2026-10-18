use crate::{DbError, blob16, open_encrypted};
use rusqlite::{Connection, OptionalExtension, params};
use std::fmt;
use std::path::Path;

const MIGRATIONS: &[&str] = &[
    "
    CREATE TABLE identity (
        id          INTEGER PRIMARY KEY CHECK (id = 1),
        account     TEXT NOT NULL,
        addressing  BLOB NOT NULL,
        created_at  INTEGER NOT NULL
    );
    CREATE TABLE contacts (
        contact_id   BLOB PRIMARY KEY,
        contact_ref  BLOB NOT NULL UNIQUE,
        display_name TEXT NOT NULL,
        card         BLOB NOT NULL,
        verified     INTEGER NOT NULL DEFAULT 0,
        state        INTEGER NOT NULL DEFAULT 0,
        paired_at    INTEGER NOT NULL,
        next_otk     INTEGER NOT NULL DEFAULT 0
    ) WITHOUT ROWID;
    CREATE TABLE olm_sessions (
        contact_id BLOB NOT NULL REFERENCES contacts(contact_id) ON DELETE CASCADE,
        session_id TEXT NOT NULL,
        pickle     TEXT NOT NULL,
        last_used  INTEGER NOT NULL,
        PRIMARY KEY (contact_id, session_id)
    ) WITHOUT ROWID;
    CREATE TABLE messages (
        message_id  BLOB PRIMARY KEY,
        contact_id  BLOB NOT NULL REFERENCES contacts(contact_id) ON DELETE CASCADE,
        outgoing    INTEGER NOT NULL,
        body        TEXT NOT NULL,
        created_at  INTEGER NOT NULL,
        state       INTEGER NOT NULL,
        envelope_id BLOB,
        ack_secret  BLOB,
        expiry_min  INTEGER
    ) WITHOUT ROWID;
    CREATE INDEX messages_conversation ON messages(contact_id, created_at);
    CREATE INDEX messages_envelope ON messages(envelope_id);
    CREATE TABLE message_events (
        id         INTEGER PRIMARY KEY,
        message_id BLOB NOT NULL REFERENCES messages(message_id) ON DELETE CASCADE,
        at         INTEGER NOT NULL,
        state      INTEGER NOT NULL
    );
    CREATE INDEX message_events_message ON message_events(message_id, at);
",
    "
    ALTER TABLE messages ADD COLUMN sent_at_ms INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE messages ADD COLUMN reply_to BLOB;
    ALTER TABLE messages ADD COLUMN forwarded INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE messages ADD COLUMN deleted INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE messages ADD COLUMN timer_secs INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE messages ADD COLUMN read_at INTEGER;
    ALTER TABLE messages ADD COLUMN expire_at INTEGER;
    ALTER TABLE messages ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE messages ADD COLUMN starred INTEGER NOT NULL DEFAULT 0;
    CREATE INDEX messages_expire ON messages(expire_at) WHERE expire_at IS NOT NULL;
    ALTER TABLE contacts ADD COLUMN blocked INTEGER NOT NULL DEFAULT 0;
    CREATE TABLE reactions (
        target      BLOB NOT NULL,
        contact_id  BLOB NOT NULL REFERENCES contacts(contact_id) ON DELETE CASCADE,
        from_me     INTEGER NOT NULL,
        reaction    INTEGER NOT NULL,
        sent_at_ms  INTEGER NOT NULL,
        event_id    BLOB NOT NULL,
        received_at INTEGER NOT NULL,
        PRIMARY KEY (target, contact_id, from_me)
    ) WITHOUT ROWID;
    CREATE TABLE edits (
        target      BLOB NOT NULL,
        contact_id  BLOB NOT NULL REFERENCES contacts(contact_id) ON DELETE CASCADE,
        from_me     INTEGER NOT NULL,
        seq         INTEGER NOT NULL,
        body        TEXT NOT NULL,
        sent_at_ms  INTEGER NOT NULL,
        received_at INTEGER NOT NULL,
        PRIMARY KEY (target, contact_id, from_me)
    ) WITHOUT ROWID;
    CREATE TABLE tombstones (
        target      BLOB NOT NULL,
        contact_id  BLOB NOT NULL REFERENCES contacts(contact_id) ON DELETE CASCADE,
        from_me     INTEGER NOT NULL,
        local       INTEGER NOT NULL,
        sent_at_ms  INTEGER NOT NULL,
        received_at INTEGER NOT NULL,
        PRIMARY KEY (target, contact_id, from_me)
    ) WITHOUT ROWID;
    CREATE TABLE drafts (
        contact_id BLOB PRIMARY KEY REFERENCES contacts(contact_id) ON DELETE CASCADE,
        body       TEXT NOT NULL,
        updated_at INTEGER NOT NULL
    ) WITHOUT ROWID;
    CREATE TABLE conversation_settings (
        contact_id       BLOB PRIMARY KEY REFERENCES contacts(contact_id) ON DELETE CASCADE,
        timer_secs       INTEGER NOT NULL,
        timer_changed_ms INTEGER NOT NULL
    ) WITHOUT ROWID;
    CREATE VIRTUAL TABLE messages_fts USING fts5(body, message_id UNINDEXED, contact_id UNINDEXED);
",
    "
    CREATE TABLE clock (id INTEGER PRIMARY KEY CHECK (id = 1), hlc_ms INTEGER NOT NULL);
",
    "
    CREATE TABLE groups (
        group_id   BLOB PRIMARY KEY,
        name       TEXT NOT NULL,
        creator_fp BLOB NOT NULL,
        epoch      INTEGER NOT NULL,
        left_group INTEGER NOT NULL DEFAULT 0,
        created_at INTEGER NOT NULL
    ) WITHOUT ROWID;
    CREATE TABLE group_members (
        group_id    BLOB NOT NULL REFERENCES groups(group_id) ON DELETE CASCADE,
        fingerprint BLOB NOT NULL,
        name        TEXT NOT NULL,
        position    INTEGER NOT NULL,
        PRIMARY KEY (group_id, fingerprint)
    ) WITHOUT ROWID;
    CREATE TABLE group_messages (
        group_msg_id BLOB PRIMARY KEY,
        group_id     BLOB NOT NULL REFERENCES groups(group_id) ON DELETE CASCADE,
        author_fp    BLOB NOT NULL,
        outgoing     INTEGER NOT NULL,
        body         TEXT NOT NULL,
        reply_to     BLOB,
        created_at   INTEGER NOT NULL,
        sent_at_ms   INTEGER NOT NULL
    ) WITHOUT ROWID;
    CREATE INDEX group_messages_order ON group_messages(group_id, created_at);
    CREATE TABLE group_pending (
        group_msg_id BLOB NOT NULL,
        contact_id   BLOB NOT NULL REFERENCES contacts(contact_id) ON DELETE CASCADE,
        group_id     BLOB NOT NULL,
        epoch        INTEGER NOT NULL,
        reply_to     BLOB,
        body         TEXT NOT NULL,
        sent_at_ms   INTEGER NOT NULL,
        received_at  INTEGER NOT NULL,
        PRIMARY KEY (group_msg_id, contact_id)
    ) WITHOUT ROWID;
    CREATE INDEX group_pending_group ON group_pending(group_id);
    CREATE TABLE group_copies (
        copy_id      BLOB PRIMARY KEY,
        group_msg_id BLOB NOT NULL,
        contact_id   BLOB NOT NULL,
        envelope_id  BLOB NOT NULL,
        ack_secret   BLOB NOT NULL,
        expiry_min   INTEGER NOT NULL,
        delivered    INTEGER NOT NULL DEFAULT 0
    ) WITHOUT ROWID;
    CREATE INDEX group_copies_message ON group_copies(group_msg_id);
    CREATE INDEX group_copies_envelope ON group_copies(envelope_id);
",
    "
    CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID;
",
    "
    CREATE TABLE attachments (
        message_id    BLOB PRIMARY KEY,
        attachment_id BLOB NOT NULL,
        contact_id    BLOB NOT NULL REFERENCES contacts(contact_id) ON DELETE CASCADE,
        outgoing      INTEGER NOT NULL,
        kind          INTEGER NOT NULL,
        name          TEXT NOT NULL,
        total_len     INTEGER NOT NULL,
        sha256        BLOB NOT NULL,
        key           BLOB NOT NULL,
        data          BLOB
    ) WITHOUT ROWID;
    CREATE UNIQUE INDEX attachments_by_id ON attachments(contact_id, attachment_id);
    CREATE TABLE attachment_chunks (
        contact_id    BLOB NOT NULL REFERENCES contacts(contact_id) ON DELETE CASCADE,
        attachment_id BLOB NOT NULL,
        idx           INTEGER NOT NULL,
        data          BLOB NOT NULL,
        received_at   INTEGER NOT NULL,
        PRIMARY KEY (contact_id, attachment_id, idx)
    ) WITHOUT ROWID;
",
    // v7 (2026-10-18): last time each contact was proven nearby (ADR-033).
    "
    CREATE TABLE contact_seen (
        contact_id BLOB PRIMARY KEY REFERENCES contacts(contact_id) ON DELETE CASCADE,
        last_seen  INTEGER NOT NULL
    ) WITHOUT ROWID;
",
    // v8 (2026-10-18): shared locations (ADR-034) and polls (ADR-035). Polls are
    // keyed by the message id they show under (a chat message or a group message).
    "
    CREATE TABLE locations (
        message_id  BLOB PRIMARY KEY,
        lat_e7      INTEGER NOT NULL,
        lon_e7      INTEGER NOT NULL,
        accuracy_m  INTEGER NOT NULL,
        captured_ms INTEGER NOT NULL
    ) WITHOUT ROWID;
    CREATE TABLE polls (
        poll_id  BLOB PRIMARY KEY,
        question TEXT NOT NULL,
        options  TEXT NOT NULL,
        multi    INTEGER NOT NULL
    ) WITHOUT ROWID;
    CREATE TABLE poll_votes (
        poll_id BLOB NOT NULL,
        voter   BLOB NOT NULL,
        choices INTEGER NOT NULL,
        at      INTEGER NOT NULL,
        PRIMARY KEY (poll_id, voter)
    ) WITHOUT ROWID;
",
    // v9 (2026-10-18): introductions (ADR-037). role 0 = I introduced two
    // contacts; role 1 = a contact introduced me to someone.
    "
    CREATE TABLE intros (
        intro_id    BLOB PRIMARY KEY,
        role        INTEGER NOT NULL,
        introducer  BLOB,
        other_name  TEXT NOT NULL DEFAULT '',
        other_fp    BLOB,
        a_contact   BLOB,
        b_contact   BLOB,
        card_a      BLOB,
        card_b      BLOB,
        state       INTEGER NOT NULL,
        message_id  BLOB,
        created_at  INTEGER NOT NULL
    ) WITHOUT ROWID;
    CREATE INDEX intros_message ON intros(message_id);
",
];

/// Partial attachment bytes held per contact (ADR-026); further chunks wait
/// until earlier attachments complete or expire.
pub const MAX_PARTIAL_ATTACHMENT_BYTES: i64 = 256 * 1024;

/// An attachment's metadata (ADR-026). `data` is set once every chunk arrived
/// and the SHA-256 matched (always set for my own attachments).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentRow {
    pub message_id: [u8; 16],
    pub attachment_id: [u8; 16],
    pub contact_id: [u8; 16],
    pub outgoing: bool,
    pub kind: u8,
    pub name: String,
    pub total_len: u32,
    pub sha256: [u8; 32],
    /// One-time key the chunks are sealed with.
    pub key: [u8; 32],
    /// Chunks held so far (equals the chunk count when complete).
    pub received_chunks: u32,
    pub complete: bool,
}

/// Group messages held because their group state has not arrived (or is older
/// than the sender's) are capped per contact and expire like orphan events.
pub const MAX_GROUP_PENDING_PER_CONTACT: i64 = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupRow {
    pub group_id: [u8; 16],
    pub name: String,
    pub creator_fp: [u8; 32],
    pub epoch: u32,
    pub left: bool,
    /// (fingerprint, name) in roster order.
    pub members: Vec<([u8; 32], String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupMessageRow {
    pub group_msg_id: [u8; 16],
    pub group_id: [u8; 16],
    pub author_fp: [u8; 32],
    pub outgoing: bool,
    pub body: String,
    pub reply_to: Option<[u8; 16]>,
    pub created_at: i64,
    pub sent_at_ms: i64,
}

/// A group message received before it could be accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingGroupMessage {
    pub contact_id: [u8; 16],
    pub epoch: u32,
    pub message: GroupMessageRow,
}

/// Outcome of applying a received group state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupApply {
    Applied,
    /// Not newer than what we hold (replay or reordering): ignored.
    Stale,
    /// Sent by someone other than the group's creator: ignored.
    NotCreator,
}

/// A contact may edit its message for this long after sending (sender clock).
pub const EDIT_WINDOW_MS: i64 = 24 * 60 * 60 * 1000;
/// A sender may request deletion for everyone for this long after sending.
pub const DELETE_WINDOW_MS: i64 = 72 * 60 * 60 * 1000;
/// Events whose target never arrives are dropped after the maximum envelope
/// lifetime plus clock skew (7 d + 2 h).
pub const ORPHAN_TTL_SECS: i64 = (7 * 24 + 2) * 60 * 60;
/// Bound on stored events per contact whose target has not arrived.
pub const MAX_ORPHANS_PER_CONTACT: i64 = 512;

const SELECT_MESSAGE: &str = "
    SELECT m.message_id, m.contact_id, m.outgoing,
           CASE WHEN m.deleted THEN '' ELSE COALESCE(e.body, m.body) END,
           m.created_at, m.state, m.envelope_id, m.ack_secret, m.expiry_min,
           m.sent_at_ms, m.reply_to, m.forwarded, m.deleted, e.body IS NOT NULL AND NOT m.deleted,
           m.timer_secs, m.read_at, m.expire_at, m.pinned, m.starred
    FROM messages m
    LEFT JOIN edits e ON e.target = m.message_id AND e.contact_id = m.contact_id
         AND e.from_me = m.outgoing AND e.sent_at_ms >= m.sent_at_ms
         AND e.sent_at_ms - m.sent_at_ms <= 86400000";

/// Sender- and recipient-side message states (Doc 18 §7.4). The numeric values are persisted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessageState {
    #[default]
    Queued = 0,
    SentToNearby = 1,
    AcceptedByRelay = 2,
    Acknowledged = 3,
    Read = 4,
    Expired = 5,
    DeliveryUnknown = 6,
    Failed = 7,
    Received = 20,
}

impl MessageState {
    fn from_i64(v: i64) -> Result<Self, DbError> {
        Ok(match v {
            0 => Self::Queued,
            1 => Self::SentToNearby,
            2 => Self::AcceptedByRelay,
            3 => Self::Acknowledged,
            4 => Self::Read,
            5 => Self::Expired,
            6 => Self::DeliveryUnknown,
            7 => Self::Failed,
            20 => Self::Received,
            _ => return Err(DbError::Corrupt("message state")),
        })
    }

    /// Allowed transitions. `Acknowledged` is terminal except for `Read`, so a
    /// late custody or expiry event can never downgrade a confirmed delivery.
    /// Authenticated proof that arrives after our copies expired still upgrades
    /// `Expired`/`DeliveryUnknown`: expiry means we stopped trying, not that the
    /// message was lost.
    pub fn can_become(self, next: Self) -> bool {
        use MessageState::*;
        matches!(
            (self, next),
            (
                Queued,
                SentToNearby | AcceptedByRelay | Acknowledged | Expired | Failed
            ) | (
                SentToNearby,
                Queued | AcceptedByRelay | Acknowledged | DeliveryUnknown | Failed
            ) | (AcceptedByRelay, Acknowledged | DeliveryUnknown | Failed)
                | (Acknowledged, Read)
                | (Expired | DeliveryUnknown, Queued | Acknowledged)
        )
    }
}

/// A shared location (ADR-034).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocationRow {
    pub lat_e7: i32,
    pub lon_e7: i32,
    pub accuracy_m: u16,
    pub captured_ms: u64,
}

/// A poll's question and options (ADR-035).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PollRow {
    pub question: String,
    pub options: Vec<String>,
    pub multi: bool,
}

/// An introduction (ADR-037). Role 0: I introduced `a_contact` and
/// `b_contact`; role 1: `introducer` introduced me to `other_name`/`other_fp`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IntroRow {
    pub intro_id: [u8; 16],
    pub role: u8,
    pub introducer: Option<[u8; 16]>,
    pub other_name: String,
    pub other_fp: Option<[u8; 32]>,
    pub a_contact: Option<[u8; 16]>,
    pub b_contact: Option<[u8; 16]>,
    pub card_a: Option<Vec<u8>>,
    pub card_b: Option<Vec<u8>>,
    pub state: u8,
    pub message_id: Option<[u8; 16]>,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Contact {
    pub contact_id: [u8; 16],
    pub contact_ref: [u8; 16],
    pub display_name: String,
    pub card: Vec<u8>,
    pub verified: bool,
    pub paired_at: i64,
    pub next_otk: u32,
    pub blocked: bool,
}

/// A stored message. On read, `body` is the effective text: the latest valid
/// edit applied, or empty when deleted.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MessageRow {
    pub message_id: [u8; 16],
    pub contact_id: [u8; 16],
    pub outgoing: bool,
    pub body: String,
    pub created_at: i64,
    pub state: MessageState,
    pub envelope_id: Option<[u8; 16]>,
    pub ack_secret: Option<[u8; 16]>,
    pub expiry_min: Option<u32>,
    /// Sender's clock; orders edits and reactions and bounds their windows.
    pub sent_at_ms: i64,
    pub reply_to: Option<[u8; 16]>,
    pub forwarded: bool,
    pub deleted: bool,
    pub edited: bool,
    pub timer_secs: u32,
    pub read_at: Option<i64>,
    pub expire_at: Option<i64>,
    pub pinned: bool,
    pub starred: bool,
}

/// A reaction currently in force on a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reaction {
    pub from_me: bool,
    /// 1..=8, index into `vl_proto::REACTIONS`.
    pub reaction: u8,
}

/// Who authored a reaction, edit or delete event, and when (sender clock).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventMeta {
    pub target: [u8; 16],
    pub contact_id: [u8; 16],
    pub from_me: bool,
    pub sent_at_ms: i64,
    pub received_at: i64,
}

/// Class C database.
pub struct ConversationDb {
    conn: Connection,
}

impl fmt::Debug for ConversationDb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ConversationDb")
    }
}

fn blob32(bytes: &[u8]) -> Result<[u8; 32], DbError> {
    bytes
        .try_into()
        .map_err(|_| DbError::Corrupt("32-byte key"))
}

fn opt16(v: Option<Vec<u8>>) -> Result<Option<[u8; 16]>, DbError> {
    v.map(|b| blob16(&b)).transpose()
}

impl ConversationDb {
    pub fn open(path: &Path, key: &[u8; 32]) -> Result<Self, DbError> {
        Ok(Self {
            conn: open_encrypted(path, key, MIGRATIONS)?,
        })
    }

    pub fn save_identity(
        &mut self,
        account_pickle: &str,
        addressing: &[u8; 32],
        now: i64,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO identity (id, account, addressing, created_at) VALUES (1, ?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET account = excluded.account, addressing = excluded.addressing",
            params![account_pickle, addressing, now],
        )?;
        Ok(())
    }

    pub fn identity(&self) -> Result<Option<(String, [u8; 32])>, DbError> {
        let row: Option<(String, Vec<u8>)> = self
            .conn
            .query_row(
                "SELECT account, addressing FROM identity WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        row.map(|(a, k)| {
            Ok((
                a,
                k.try_into()
                    .map_err(|_| DbError::Corrupt("addressing key"))?,
            ))
        })
        .transpose()
    }

    pub fn add_contact(&mut self, c: &Contact) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO contacts (contact_id, contact_ref, display_name, card, verified, paired_at, next_otk, blocked)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![c.contact_id, c.contact_ref, c.display_name, c.card, c.verified, c.paired_at, c.next_otk, c.blocked],
        )?;
        Ok(())
    }

    pub fn contacts(&self) -> Result<Vec<Contact>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT contact_id, contact_ref, display_name, card, verified, paired_at, next_otk, blocked FROM contacts ORDER BY display_name",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, Vec<u8>>(0)?,
                r.get::<_, Vec<u8>>(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
            ))
        })?;
        rows.map(|r| {
            let (id, cref, display_name, card, verified, paired_at, next_otk, blocked) = r?;
            Ok(Contact {
                contact_id: blob16(&id)?,
                contact_ref: blob16(&cref)?,
                display_name,
                card,
                verified,
                paired_at,
                next_otk,
                blocked,
            })
        })
        .collect()
    }

    pub fn set_verified(&mut self, contact_id: &[u8; 16], verified: bool) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE contacts SET verified = ?2 WHERE contact_id = ?1",
            params![contact_id, verified],
        )?;
        Ok(())
    }

    pub fn set_blocked(&mut self, contact_id: &[u8; 16], blocked: bool) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE contacts SET blocked = ?2 WHERE contact_id = ?1",
            params![contact_id, blocked],
        )?;
        Ok(())
    }

    pub fn save_session(
        &mut self,
        contact_id: &[u8; 16],
        session_id: &str,
        pickle: &str,
        now: i64,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO olm_sessions (contact_id, session_id, pickle, last_used) VALUES (?1, ?2, ?3, ?4)",
            params![contact_id, session_id, pickle, now],
        )?;
        Ok(())
    }

    /// Sessions for a contact, most recently used first (Olm tries each on decrypt).
    pub fn sessions(&self, contact_id: &[u8; 16]) -> Result<Vec<(String, String)>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT session_id, pickle FROM olm_sessions WHERE contact_id = ?1 ORDER BY last_used DESC")?;
        let rows = stmt.query_map([contact_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
        rows.collect::<Result<_, _>>().map_err(Into::into)
    }

    /// Stores a message. A delete request that arrived before it turns it into a
    /// deleted placeholder; a local delete-for-me suppresses it entirely.
    /// Returns false if it was already present or suppressed.
    pub fn insert_message(&mut self, m: &MessageRow) -> Result<bool, DbError> {
        let tx = self.conn.transaction()?;
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM messages WHERE message_id = ?1)",
            [m.message_id],
            |r| r.get(0),
        )?;
        let tomb: Option<(bool, i64)> = tx
            .query_row(
                "SELECT local, sent_at_ms FROM tombstones
                 WHERE target = ?1 AND contact_id = ?2 AND (local OR from_me = ?3)",
                params![m.message_id, m.contact_id, m.outgoing],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if exists || matches!(tomb, Some((true, _))) {
            return Ok(false);
        }
        let deleted = m.deleted
            || tomb.is_some_and(|(_, at)| (0..=DELETE_WINDOW_MS).contains(&(at - m.sent_at_ms)));
        tx.execute(
            "INSERT INTO messages (message_id, contact_id, outgoing, body, created_at, state, envelope_id, ack_secret, expiry_min,
                                   sent_at_ms, reply_to, forwarded, deleted, timer_secs, read_at, expire_at, pinned, starred)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                m.message_id, m.contact_id, m.outgoing, if deleted { "" } else { m.body.as_str() },
                m.created_at, m.state as i64, m.envelope_id, m.ack_secret, m.expiry_min,
                m.sent_at_ms, m.reply_to, m.forwarded, deleted, m.timer_secs, m.read_at, m.expire_at,
                m.pinned, m.starred
            ],
        )?;
        tx.execute(
            "INSERT INTO message_events (message_id, at, state) VALUES (?1, ?2, ?3)",
            params![m.message_id, m.created_at, m.state as i64],
        )?;
        if deleted {
            purge_annotations(&tx, &m.message_id)?;
        }
        reindex(&tx, &m.message_id)?;
        tx.commit()?;
        Ok(true)
    }

    /// Applies a state change only if [`MessageState::can_become`] allows it. Returns whether it changed.
    pub fn transition(
        &mut self,
        message_id: &[u8; 16],
        next: MessageState,
        at: i64,
    ) -> Result<bool, DbError> {
        let tx = self.conn.transaction()?;
        let current: Option<i64> = tx
            .query_row(
                "SELECT state FROM messages WHERE message_id = ?1",
                [message_id],
                |r| r.get(0),
            )
            .optional()?;
        let Some(current) = current else {
            return Ok(false);
        };
        if !MessageState::from_i64(current)?.can_become(next) {
            return Ok(false);
        }
        tx.execute(
            "UPDATE messages SET state = ?2 WHERE message_id = ?1",
            params![message_id, next as i64],
        )?;
        tx.execute(
            "INSERT INTO message_events (message_id, at, state) VALUES (?1, ?2, ?3)",
            params![message_id, at, next as i64],
        )?;
        tx.commit()?;
        Ok(true)
    }

    pub fn message_by_envelope(&self, envelope_id: &[u8; 16]) -> Result<Option<[u8; 16]>, DbError> {
        let id: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT message_id FROM messages WHERE envelope_id = ?1",
                [envelope_id],
                |r| r.get(0),
            )
            .optional()?;
        id.map(|b| blob16(&b)).transpose()
    }

    pub fn message(&self, message_id: &[u8; 16]) -> Result<Option<MessageRow>, DbError> {
        let sql = format!("{SELECT_MESSAGE} WHERE m.message_id = ?1");
        self.conn
            .query_row(&sql, [message_id], row_tuple)
            .optional()?
            .map(message_from)
            .transpose()
    }

    /// Newest first.
    pub fn conversation(
        &self,
        contact_id: &[u8; 16],
        limit: u32,
    ) -> Result<Vec<MessageRow>, DbError> {
        let sql = format!(
            "{SELECT_MESSAGE} WHERE m.contact_id = ?1 ORDER BY m.created_at DESC, m.sent_at_ms DESC LIMIT ?2"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![contact_id, limit], row_tuple)?;
        rows.map(|r| message_from(r?)).collect()
    }

    /// Renames a contact on this phone only.
    pub fn rename_contact(&mut self, contact_id: &[u8; 16], name: &str) -> Result<bool, DbError> {
        Ok(self.conn.execute(
            "UPDATE contacts SET display_name = ?2 WHERE contact_id = ?1",
            params![contact_id, name],
        )? > 0)
    }

    /// Deletes a contact and everything exchanged with them (messages,
    /// sessions, drafts, attachments, search entries). Returns its `contact_ref`
    /// so the caller can drop the relay-side keys and tags.
    pub fn delete_contact(&mut self, contact_id: &[u8; 16]) -> Result<Option<[u8; 16]>, DbError> {
        let tx = self.conn.transaction()?;
        let contact_ref: Option<Vec<u8>> = tx
            .query_row(
                "SELECT contact_ref FROM contacts WHERE contact_id = ?1",
                [contact_id],
                |r| r.get(0),
            )
            .optional()?;
        let Some(contact_ref) = contact_ref else {
            return Ok(None);
        };
        tx.execute(
            "DELETE FROM messages_fts WHERE contact_id = ?1",
            [contact_id],
        )?;
        tx.execute(
            "DELETE FROM group_copies WHERE contact_id = ?1",
            [contact_id],
        )?;
        tx.execute("DELETE FROM contacts WHERE contact_id = ?1", [contact_id])?;
        tx.commit()?;
        Ok(Some(blob16(&contact_ref)?))
    }

    /// Re-pairing in person (same identity key): new card, fresh one-time keys,
    /// and the old Olm sessions dropped. History and verification are kept.
    pub fn replace_contact_card(
        &mut self,
        contact_id: &[u8; 16],
        card: &[u8],
    ) -> Result<(), DbError> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE contacts SET card = ?2, next_otk = 0 WHERE contact_id = ?1",
            params![contact_id, card],
        )?;
        tx.execute(
            "DELETE FROM olm_sessions WHERE contact_id = ?1",
            [contact_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Points an outgoing message at its new envelope after a resend.
    pub fn set_message_envelope(
        &mut self,
        message_id: &[u8; 16],
        envelope_id: &[u8; 16],
        ack_secret: &[u8; 16],
        expiry_min: u32,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE messages SET envelope_id = ?2, ack_secret = ?3, expiry_min = ?4 WHERE message_id = ?1",
            params![message_id, envelope_id, ack_secret, expiry_min],
        )?;
        Ok(())
    }

    /// Records an attachment's metadata; `data` is given for my own attachments.
    pub fn put_attachment(
        &mut self,
        a: &AttachmentRow,
        data: Option<&[u8]>,
    ) -> Result<bool, DbError> {
        Ok(self.conn.execute(
            "INSERT OR IGNORE INTO attachments
                 (message_id, attachment_id, contact_id, outgoing, kind, name, total_len, sha256, key, data)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![a.message_id, a.attachment_id, a.contact_id, a.outgoing, a.kind, a.name, a.total_len, a.sha256, a.key, data],
        )? > 0)
    }

    /// Stores one received chunk unless it is a duplicate, belongs to a
    /// completed attachment, or the contact's partial-bytes quota is full.
    pub fn add_chunk(
        &mut self,
        contact_id: &[u8; 16],
        attachment_id: &[u8; 16],
        index: u16,
        data: &[u8],
        now: i64,
    ) -> Result<bool, DbError> {
        let complete: bool = self
            .conn
            .query_row(
                "SELECT data IS NOT NULL FROM attachments WHERE contact_id = ?1 AND attachment_id = ?2",
                params![contact_id, attachment_id],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or(false);
        if complete {
            return Ok(false);
        }
        let held: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(length(data)), 0) FROM attachment_chunks WHERE contact_id = ?1",
            [contact_id],
            |r| r.get(0),
        )?;
        let len = i64::try_from(data.len()).unwrap_or(i64::MAX);
        if held.saturating_add(len) > MAX_PARTIAL_ATTACHMENT_BYTES {
            return Ok(false);
        }
        Ok(self.conn.execute(
            "INSERT OR IGNORE INTO attachment_chunks (contact_id, attachment_id, idx, data, received_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![contact_id, attachment_id, index, data, now],
        )? > 0)
    }

    /// If every chunk of a known, incomplete attachment is present, returns its
    /// bytes in order (the caller verifies the SHA-256 before completing it).
    pub fn assembled_attachment(
        &self,
        contact_id: &[u8; 16],
        attachment_id: &[u8; 16],
        chunk_count: u32,
    ) -> Result<Option<Vec<Vec<u8>>>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT idx, data FROM attachment_chunks
             WHERE contact_id = ?1 AND attachment_id = ?2 AND idx < ?3 ORDER BY idx",
        )?;
        let rows = stmt.query_map(params![contact_id, attachment_id, chunk_count], |r| {
            Ok((r.get::<_, u32>(0)?, r.get::<_, Vec<u8>>(1)?))
        })?;
        let mut out = Vec::new();
        let mut expected = 0u32;
        for row in rows {
            let (idx, data) = row?;
            if idx != expected {
                return Ok(None);
            }
            out.push(data);
            expected += 1;
        }
        Ok((expected == chunk_count).then_some(out))
    }

    /// Stores the verified bytes (or, with `None`, discards chunks that failed
    /// verification) and frees the chunk rows.
    pub fn finish_attachment(
        &mut self,
        contact_id: &[u8; 16],
        attachment_id: &[u8; 16],
        data: Option<&[u8]>,
    ) -> Result<(), DbError> {
        let tx = self.conn.transaction()?;
        if let Some(data) = data {
            tx.execute(
                "UPDATE attachments SET data = ?3 WHERE contact_id = ?1 AND attachment_id = ?2",
                params![contact_id, attachment_id, data],
            )?;
        }
        tx.execute(
            "DELETE FROM attachment_chunks WHERE contact_id = ?1 AND attachment_id = ?2",
            params![contact_id, attachment_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn attachment(&self, message_id: &[u8; 16]) -> Result<Option<AttachmentRow>, DbError> {
        let row = self
            .conn
            .query_row(
                "SELECT a.message_id, a.attachment_id, a.contact_id, a.outgoing, a.kind, a.name,
                        a.total_len, a.sha256, a.key, a.data IS NOT NULL,
                        (SELECT COUNT(*) FROM attachment_chunks c
                          WHERE c.contact_id = a.contact_id AND c.attachment_id = a.attachment_id)
                 FROM attachments a WHERE a.message_id = ?1",
                [message_id],
                |r| {
                    let blobs: [Vec<u8>; 5] =
                        [r.get(0)?, r.get(1)?, r.get(2)?, r.get(7)?, r.get(8)?];
                    let total_len: u32 = r.get(6)?;
                    let complete: bool = r.get(9)?;
                    let chunks: u32 = r.get(10)?;
                    Ok((
                        blobs,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        total_len,
                        complete,
                        chunks,
                    ))
                },
            )
            .optional()?;
        let Some(([mid, aid, cid, sha, key], outgoing, kind, name, total_len, complete, chunks)) =
            row
        else {
            return Ok(None);
        };
        Ok(Some(AttachmentRow {
            message_id: blob16(&mid)?,
            attachment_id: blob16(&aid)?,
            contact_id: blob16(&cid)?,
            outgoing,
            kind,
            name,
            total_len,
            sha256: blob32(&sha)?,
            key: blob32(&key)?,
            received_chunks: if complete {
                vl_proto::chunk_count_for(total_len)
            } else {
                chunks
            },
            complete,
        }))
    }

    pub fn attachment_by_id(
        &self,
        contact_id: &[u8; 16],
        attachment_id: &[u8; 16],
    ) -> Result<Option<AttachmentRow>, DbError> {
        let mid: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT message_id FROM attachments WHERE contact_id = ?1 AND attachment_id = ?2",
                params![contact_id, attachment_id],
                |r| r.get(0),
            )
            .optional()?;
        match mid {
            Some(m) => self.attachment(&blob16(&m)?),
            None => Ok(None),
        }
    }

    pub fn attachment_data(&self, message_id: &[u8; 16]) -> Result<Option<Vec<u8>>, DbError> {
        Ok(self
            .conn
            .query_row(
                "SELECT data FROM attachments WHERE message_id = ?1",
                [message_id],
                |r| r.get::<_, Option<Vec<u8>>>(0),
            )
            .optional()?
            .flatten())
    }

    /// Received messages not yet read (deleted ones excluded).
    pub fn unread_count(&self, contact_id: &[u8; 16]) -> Result<u32, DbError> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM messages
             WHERE contact_id = ?1 AND NOT outgoing AND read_at IS NULL AND NOT deleted",
            [contact_id],
            |r| r.get(0),
        )?)
    }

    /// Outgoing messages still waiting for delivery proof, newest first.
    pub fn pending_outgoing(&self) -> Result<Vec<MessageRow>, DbError> {
        let sql = format!(
            "{SELECT_MESSAGE} WHERE m.outgoing AND NOT m.deleted AND m.state IN (0, 1, 2, 6)
             ORDER BY m.created_at DESC"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], row_tuple)?;
        rows.map(|r| message_from(r?)).collect()
    }

    /// State history of one message, oldest first.
    pub fn message_events(
        &self,
        message_id: &[u8; 16],
    ) -> Result<Vec<(i64, MessageState)>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT at, state FROM message_events WHERE message_id = ?1 ORDER BY at, id",
        )?;
        let rows = stmt.query_map([message_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
        rows.map(|r| {
            let (at, state): (i64, i64) = r?;
            Ok((at, MessageState::from_i64(state)?))
        })
        .collect()
    }

    pub fn put_location(&mut self, message_id: &[u8; 16], l: &LocationRow) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO locations (message_id, lat_e7, lon_e7, accuracy_m, captured_ms)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                message_id,
                l.lat_e7,
                l.lon_e7,
                l.accuracy_m,
                i64::try_from(l.captured_ms).unwrap_or(i64::MAX)
            ],
        )?;
        Ok(())
    }

    pub fn location(&self, message_id: &[u8; 16]) -> Result<Option<LocationRow>, DbError> {
        Ok(self
            .conn
            .query_row(
                "SELECT lat_e7, lon_e7, accuracy_m, captured_ms FROM locations WHERE message_id = ?1",
                [message_id],
                |r| {
                    Ok(LocationRow {
                        lat_e7: r.get(0)?,
                        lon_e7: r.get(1)?,
                        accuracy_m: r.get(2)?,
                        captured_ms: u64::try_from(r.get::<_, i64>(3)?).unwrap_or(0),
                    })
                },
            )
            .optional()?)
    }

    /// Options are stored newline-separated; the protocol forbids empty ones
    /// and the core strips newlines from them before sending.
    pub fn put_poll(&mut self, poll_id: &[u8; 16], p: &PollRow) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO polls (poll_id, question, options, multi) VALUES (?1, ?2, ?3, ?4)",
            params![poll_id, p.question, p.options.join("\n"), p.multi],
        )?;
        Ok(())
    }

    pub fn poll(&self, poll_id: &[u8; 16]) -> Result<Option<PollRow>, DbError> {
        Ok(self
            .conn
            .query_row(
                "SELECT question, options, multi FROM polls WHERE poll_id = ?1",
                [poll_id],
                |r| {
                    Ok(PollRow {
                        question: r.get(0)?,
                        options: r
                            .get::<_, String>(1)?
                            .split('\n')
                            .map(String::from)
                            .collect(),
                        multi: r.get(2)?,
                    })
                },
            )
            .optional()?)
    }

    /// A voter's whole current choice; 0 withdraws the vote.
    pub fn set_vote(
        &mut self,
        poll_id: &[u8; 16],
        voter: &[u8; 32],
        choices: u16,
        at: i64,
    ) -> Result<(), DbError> {
        if choices == 0 {
            self.conn.execute(
                "DELETE FROM poll_votes WHERE poll_id = ?1 AND voter = ?2",
                params![poll_id, voter],
            )?;
        } else {
            self.conn.execute(
                "INSERT INTO poll_votes (poll_id, voter, choices, at) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (poll_id, voter) DO UPDATE SET choices = excluded.choices, at = excluded.at",
                params![poll_id, voter, choices, at],
            )?;
        }
        Ok(())
    }

    /// Every voter's current choice for a poll.
    pub fn votes(&self, poll_id: &[u8; 16]) -> Result<Vec<([u8; 32], u16)>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT voter, choices FROM poll_votes WHERE poll_id = ?1")?;
        let rows = stmt.query_map([poll_id], |r| {
            Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, u16>(1)?))
        })?;
        rows.map(|r| {
            let (v, c) = r?;
            let voter: [u8; 32] = v
                .as_slice()
                .try_into()
                .map_err(|_| DbError::Corrupt("voter"))?;
            Ok((voter, c))
        })
        .collect()
    }

    pub fn put_intro(&mut self, i: &IntroRow) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO intros (intro_id, role, introducer, other_name, other_fp, a_contact,
                 b_contact, card_a, card_b, state, message_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                i.intro_id, i.role, i.introducer, i.other_name, i.other_fp, i.a_contact, i.b_contact,
                i.card_a, i.card_b, i.state, i.message_id, i.created_at
            ],
        )?;
        Ok(())
    }

    fn intro_where(&self, clause: &str, key: &[u8]) -> Result<Option<IntroRow>, DbError> {
        let sql = format!(
            "SELECT intro_id, role, introducer, other_name, other_fp, a_contact, b_contact, card_a, card_b,
                 state, message_id, created_at FROM intros WHERE {clause} = ?1 LIMIT 1"
        );
        let row = self
            .conn
            .query_row(&sql, [key], |r| {
                Ok((
                    r.get::<_, Vec<u8>>(0)?,
                    r.get::<_, u8>(1)?,
                    r.get::<_, Option<Vec<u8>>>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, Option<Vec<u8>>>(4)?,
                    r.get::<_, Option<Vec<u8>>>(5)?,
                    r.get::<_, Option<Vec<u8>>>(6)?,
                    r.get::<_, Option<Vec<u8>>>(7)?,
                    r.get::<_, Option<Vec<u8>>>(8)?,
                    r.get::<_, u8>(9)?,
                    r.get::<_, Option<Vec<u8>>>(10)?,
                    r.get::<_, i64>(11)?,
                ))
            })
            .optional()?;
        row.map(
            |(
                id,
                role,
                introducer,
                other_name,
                fp,
                a,
                b,
                card_a,
                card_b,
                state,
                mid,
                created_at,
            )| {
                Ok(IntroRow {
                    intro_id: blob16(&id)?,
                    role,
                    introducer: opt16(introducer)?,
                    other_name,
                    other_fp: fp.map(|v| blob32(&v)).transpose()?,
                    a_contact: opt16(a)?,
                    b_contact: opt16(b)?,
                    card_a,
                    card_b,
                    state,
                    message_id: opt16(mid)?,
                    created_at,
                })
            },
        )
        .transpose()
    }

    pub fn intro(&self, intro_id: &[u8; 16]) -> Result<Option<IntroRow>, DbError> {
        self.intro_where("intro_id", intro_id)
    }

    pub fn intro_by_message(&self, message_id: &[u8; 16]) -> Result<Option<IntroRow>, DbError> {
        self.intro_where("message_id", message_id)
    }

    /// Who introduced me to this identity, if a completed introduction did.
    pub fn introducer_of(&self, fp: &[u8; 32]) -> Result<Option<[u8; 16]>, DbError> {
        let v: Option<Option<Vec<u8>>> = self
            .conn
            .query_row(
                "SELECT introducer FROM intros WHERE role = 1 AND state = 3 AND other_fp = ?1 LIMIT 1",
                [fp],
                |r| r.get(0),
            )
            .optional()?;
        v.flatten().map(|b| blob16(&b)).transpose()
    }

    /// Records that a link proved `contact_id` was in Bluetooth range at `at`
    /// (ADR-033). One timestamp per contact; never a history of meetings.
    pub fn touch_seen(&mut self, contact_id: &[u8; 16], at: i64) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO contact_seen (contact_id, last_seen) VALUES (?1, ?2)
             ON CONFLICT (contact_id) DO UPDATE SET last_seen = max(last_seen, excluded.last_seen)",
            params![contact_id, at],
        )?;
        Ok(())
    }

    /// Last time each contact was proven nearby.
    pub fn last_seen_all(&self) -> Result<std::collections::HashMap<[u8; 16], i64>, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT contact_id, last_seen FROM contact_seen")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, i64>(1)?)))?;
        rows.map(|r| {
            let (id, at) = r?;
            Ok((blob16(&id)?, at))
        })
        .collect()
    }

    /// Local app settings, kept in this encrypted database. `None` removes the key.
    pub fn set_setting(&mut self, key: &str, value: Option<&str>) -> Result<(), DbError> {
        match value {
            Some(v) => self.conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT (key) DO UPDATE SET value = excluded.value",
                params![key, v],
            )?,
            None => self
                .conn
                .execute("DELETE FROM settings WHERE key = ?1", [key])?,
        };
        Ok(())
    }

    pub fn setting(&self, key: &str) -> Result<Option<String>, DbError> {
        Ok(self
            .conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
                r.get(0)
            })
            .optional()?)
    }

    /// Records a reaction event; per (target, author) the greatest
    /// `(sent_at_ms, event_id)` wins regardless of arrival order. Reaction 0 is
    /// kept as a removal marker so an older event cannot resurrect it.
    pub fn upsert_reaction(
        &mut self,
        ev: &EventMeta,
        reaction: u8,
        event_id: &[u8; 16],
    ) -> Result<bool, DbError> {
        if !self.event_admissible(ev)? {
            return Ok(false);
        }
        let n = self.conn.execute(
            "INSERT INTO reactions (target, contact_id, from_me, reaction, sent_at_ms, event_id, received_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (target, contact_id, from_me) DO UPDATE SET
                 reaction = excluded.reaction, sent_at_ms = excluded.sent_at_ms,
                 event_id = excluded.event_id, received_at = excluded.received_at
             WHERE (excluded.sent_at_ms, excluded.event_id) > (reactions.sent_at_ms, reactions.event_id)",
            params![ev.target, ev.contact_id, ev.from_me, reaction, ev.sent_at_ms, event_id, ev.received_at],
        )?;
        Ok(n > 0)
    }

    /// Reactions in force on a message, limited to authors in its conversation.
    pub fn reactions(&self, target: &[u8; 16]) -> Result<Vec<Reaction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT r.from_me, r.reaction FROM reactions r
             JOIN messages m ON m.message_id = r.target AND m.contact_id = r.contact_id
             WHERE r.target = ?1 AND r.reaction > 0 AND NOT m.deleted ORDER BY r.from_me",
        )?;
        let rows = stmt.query_map([target], |r| {
            Ok(Reaction {
                from_me: r.get(0)?,
                reaction: r.get(1)?,
            })
        })?;
        rows.collect::<Result<_, _>>().map_err(Into::into)
    }

    /// Records an edit; per (target, author) the greatest `(seq, sent_at_ms)` wins.
    /// It takes effect only on the author's own message within [`EDIT_WINDOW_MS`].
    pub fn upsert_edit(&mut self, ev: &EventMeta, seq: u8, body: &str) -> Result<bool, DbError> {
        if !self.event_admissible(ev)? {
            return Ok(false);
        }
        let tx = self.conn.transaction()?;
        let n = tx.execute(
            "INSERT INTO edits (target, contact_id, from_me, seq, body, sent_at_ms, received_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (target, contact_id, from_me) DO UPDATE SET
                 seq = excluded.seq, body = excluded.body, sent_at_ms = excluded.sent_at_ms,
                 received_at = excluded.received_at
             WHERE (excluded.seq, excluded.sent_at_ms) > (edits.seq, edits.sent_at_ms)",
            params![
                ev.target,
                ev.contact_id,
                ev.from_me,
                seq,
                body,
                ev.sent_at_ms,
                ev.received_at
            ],
        )?;
        reindex(&tx, &ev.target)?;
        tx.commit()?;
        Ok(n > 0)
    }

    /// Highest edit sequence number recorded for a message by its author (0 if none).
    pub fn edit_seq(
        &self,
        target: &[u8; 16],
        contact_id: &[u8; 16],
        from_me: bool,
    ) -> Result<u8, DbError> {
        Ok(self
            .conn
            .query_row(
                "SELECT seq FROM edits WHERE target = ?1 AND contact_id = ?2 AND from_me = ?3",
                params![target, contact_id, from_me],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or(0))
    }

    /// Records a delete-for-everyone request. It applies to the author's own
    /// message within [`DELETE_WINDOW_MS`]; if the message has not arrived yet the
    /// tombstone waits for it.
    pub fn add_tombstone(&mut self, ev: &EventMeta) -> Result<bool, DbError> {
        if !self.event_admissible(ev)? {
            return Ok(false);
        }
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT OR IGNORE INTO tombstones (target, contact_id, from_me, local, sent_at_ms, received_at)
             VALUES (?1, ?2, ?3, 0, ?4, ?5)",
            params![ev.target, ev.contact_id, ev.from_me, ev.sent_at_ms, ev.received_at],
        )?;
        let n = tx.execute(
            "UPDATE messages SET deleted = 1, body = ''
             WHERE message_id = ?1 AND contact_id = ?2 AND outgoing = ?3 AND NOT deleted
               AND ?4 - sent_at_ms BETWEEN 0 AND ?5",
            params![
                ev.target,
                ev.contact_id,
                ev.from_me,
                ev.sent_at_ms,
                DELETE_WINDOW_MS
            ],
        )?;
        if n > 0 {
            purge_annotations(&tx, &ev.target)?;
            reindex(&tx, &ev.target)?;
        }
        tx.commit()?;
        Ok(n > 0)
    }

    /// Deletes a message from this device only and remembers it so a duplicate
    /// copy is not stored again. Copies already handed to relays cannot be recalled.
    pub fn delete_message(&mut self, message_id: &[u8; 16], now: i64) -> Result<(), DbError> {
        let tx = self.conn.transaction()?;
        let owner: Option<(Vec<u8>, bool, i64)> = tx
            .query_row(
                "SELECT contact_id, outgoing, sent_at_ms FROM messages WHERE message_id = ?1",
                [message_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        if let Some((contact_id, from_me, sent_at_ms)) = owner {
            tx.execute(
                "INSERT OR REPLACE INTO tombstones (target, contact_id, from_me, local, sent_at_ms, received_at)
                 VALUES (?1, ?2, ?3, 1, ?4, ?5)",
                params![message_id, contact_id, from_me, sent_at_ms, now],
            )?;
        }
        purge_message(&tx, message_id)?;
        tx.commit()?;
        Ok(())
    }

    /// Last hybrid-logical-clock value issued or observed by this node (0 if none).
    pub fn hlc(&self) -> Result<i64, DbError> {
        Ok(self
            .conn
            .query_row("SELECT hlc_ms FROM clock WHERE id = 1", [], |r| r.get(0))
            .optional()?
            .unwrap_or(0))
    }

    /// Raises the stored clock to `hlc_ms`; never lowers it.
    pub fn set_hlc(&mut self, hlc_ms: i64) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO clock (id, hlc_ms) VALUES (1, ?1)
             ON CONFLICT (id) DO UPDATE SET hlc_ms = max(hlc_ms, excluded.hlc_ms)",
            [hlc_ms],
        )?;
        Ok(())
    }

    /// Current disappearing-message timer for a conversation (0 = off).
    pub fn timer(&self, contact_id: &[u8; 16]) -> Result<u32, DbError> {
        Ok(self
            .conn
            .query_row(
                "SELECT timer_secs FROM conversation_settings WHERE contact_id = ?1",
                [contact_id],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or(0))
    }

    /// Last-writer-wins by the sender clock of the message that carried the value.
    pub fn set_timer(
        &mut self,
        contact_id: &[u8; 16],
        timer_secs: u32,
        changed_ms: i64,
    ) -> Result<bool, DbError> {
        let n = self.conn.execute(
            "INSERT INTO conversation_settings (contact_id, timer_secs, timer_changed_ms) VALUES (?1, ?2, ?3)
             ON CONFLICT (contact_id) DO UPDATE SET timer_secs = excluded.timer_secs,
                 timer_changed_ms = excluded.timer_changed_ms
             WHERE excluded.timer_changed_ms > conversation_settings.timer_changed_ms",
            params![contact_id, timer_secs, changed_ms],
        )?;
        Ok(n > 0)
    }

    /// Marks received messages read and starts their disappearing timers.
    pub fn mark_read(&mut self, contact_id: &[u8; 16], now: i64) -> Result<usize, DbError> {
        Ok(self.conn.execute(
            "UPDATE messages SET read_at = ?2,
                 expire_at = CASE WHEN timer_secs > 0 THEN ?2 + timer_secs END
             WHERE contact_id = ?1 AND NOT outgoing AND read_at IS NULL",
            params![contact_id, now],
        )?)
    }

    /// Removes messages whose disappearing timer has run out, and events whose
    /// target never arrived. Returns the number of messages removed.
    pub fn purge_expired(&mut self, now: i64) -> Result<usize, DbError> {
        let tx = self.conn.transaction()?;
        let ids: Vec<Vec<u8>> = {
            let mut stmt = tx.prepare("SELECT message_id FROM messages WHERE expire_at <= ?1")?;
            let rows = stmt.query_map([now], |r| r.get(0))?;
            rows.collect::<Result<_, _>>()?
        };
        for id in &ids {
            purge_message(&tx, &blob16(id)?)?;
        }
        let horizon = now - ORPHAN_TTL_SECS;
        tx.execute(
            "DELETE FROM group_pending WHERE received_at < ?1",
            [horizon],
        )?;
        tx.execute(
            "DELETE FROM attachment_chunks WHERE received_at < ?1",
            [horizon],
        )?;
        for table in ["reactions", "edits", "tombstones"] {
            tx.execute(
                &format!(
                    "DELETE FROM {table} WHERE received_at < ?1
                     AND target NOT IN (SELECT message_id FROM messages)"
                ),
                [horizon],
            )?;
        }
        tx.commit()?;
        Ok(ids.len())
    }

    /// Full-text search over effective message text, newest first. User input
    /// is reduced to quoted prefix terms, so FTS5 query syntax cannot be injected.
    pub fn search(
        &self,
        query: &str,
        contact_id: Option<&[u8; 16]>,
        limit: u32,
    ) -> Result<Vec<MessageRow>, DbError> {
        let terms: Vec<String> = query
            .split_whitespace()
            .map(|t| t.replace('"', ""))
            .filter(|t| t.chars().any(char::is_alphanumeric))
            .map(|t| format!("\"{t}\"*"))
            .collect();
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let mut stmt = self.conn.prepare(
            "SELECT message_id FROM messages_fts WHERE messages_fts MATCH ?1
             AND (?2 IS NULL OR contact_id = ?2) ORDER BY rank LIMIT ?3",
        )?;
        let ids: Vec<Vec<u8>> = stmt
            .query_map(params![terms.join(" "), contact_id, limit], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(m) = self.message(&blob16(&id)?)? {
                out.push(m);
            }
        }
        out.sort_by_key(|m| std::cmp::Reverse(m.created_at));
        Ok(out)
    }

    pub fn save_draft(
        &mut self,
        contact_id: &[u8; 16],
        body: &str,
        now: i64,
    ) -> Result<(), DbError> {
        if body.is_empty() {
            self.conn
                .execute("DELETE FROM drafts WHERE contact_id = ?1", [contact_id])?;
        } else {
            self.conn.execute(
                "INSERT INTO drafts (contact_id, body, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT (contact_id) DO UPDATE SET body = excluded.body, updated_at = excluded.updated_at",
                params![contact_id, body, now],
            )?;
        }
        Ok(())
    }

    pub fn draft(&self, contact_id: &[u8; 16]) -> Result<Option<String>, DbError> {
        Ok(self
            .conn
            .query_row(
                "SELECT body FROM drafts WHERE contact_id = ?1",
                [contact_id],
                |r| r.get(0),
            )
            .optional()?)
    }

    pub fn set_pinned(&mut self, message_id: &[u8; 16], pinned: bool) -> Result<bool, DbError> {
        Ok(self.conn.execute(
            "UPDATE messages SET pinned = ?2 WHERE message_id = ?1 AND NOT deleted",
            params![message_id, pinned],
        )? > 0)
    }

    pub fn set_starred(&mut self, message_id: &[u8; 16], starred: bool) -> Result<bool, DbError> {
        Ok(self.conn.execute(
            "UPDATE messages SET starred = ?2 WHERE message_id = ?1 AND NOT deleted",
            params![message_id, starred],
        )? > 0)
    }

    /// Pinned or starred messages in a conversation (or all), newest first.
    pub fn flagged(
        &self,
        contact_id: Option<&[u8; 16]>,
        pinned: bool,
    ) -> Result<Vec<MessageRow>, DbError> {
        let column = if pinned { "pinned" } else { "starred" };
        let sql = format!(
            "{SELECT_MESSAGE} WHERE m.{column} AND (?1 IS NULL OR m.contact_id = ?1) ORDER BY m.created_at DESC"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![contact_id], row_tuple)?;
        rows.map(|r| message_from(r?)).collect()
    }

    pub fn group(&self, group_id: &[u8; 16]) -> Result<Option<GroupRow>, DbError> {
        let head: Option<(String, Vec<u8>, u32, bool)> = self
            .conn
            .query_row(
                "SELECT name, creator_fp, epoch, left_group FROM groups WHERE group_id = ?1",
                [group_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        let Some((name, creator, epoch, left)) = head else {
            return Ok(None);
        };
        let mut stmt = self.conn.prepare_cached(
            "SELECT fingerprint, name FROM group_members WHERE group_id = ?1 ORDER BY position",
        )?;
        let members = stmt
            .query_map([group_id], |r| {
                Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, String>(1)?))
            })?
            .map(|r| {
                let (fp, n) = r?;
                Ok((blob32(&fp)?, n))
            })
            .collect::<Result<Vec<_>, DbError>>()?;
        Ok(Some(GroupRow {
            group_id: *group_id,
            name,
            creator_fp: blob32(&creator)?,
            epoch,
            left,
            members,
        }))
    }

    pub fn groups(&self) -> Result<Vec<GroupRow>, DbError> {
        let ids: Vec<Vec<u8>> = {
            let mut stmt = self
                .conn
                .prepare("SELECT group_id FROM groups ORDER BY created_at DESC")?;
            let rows = stmt.query_map([], |r| r.get(0))?;
            rows.collect::<Result<_, _>>()?
        };
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(g) = self.group(&blob16(&id)?)? {
                out.push(g);
            }
        }
        Ok(out)
    }

    /// Applies a full roster. Only the creator may change a known group, and
    /// only to a strictly newer epoch (single writer, total order).
    pub fn apply_group_state(
        &mut self,
        g: &GroupRow,
        sender_fp: &[u8; 32],
        now: i64,
    ) -> Result<GroupApply, DbError> {
        if let Some(existing) = self.group(&g.group_id)? {
            if &existing.creator_fp != sender_fp {
                return Ok(GroupApply::NotCreator);
            }
            if g.epoch <= existing.epoch {
                return Ok(GroupApply::Stale);
            }
        } else if &g.creator_fp != sender_fp {
            return Ok(GroupApply::NotCreator);
        }
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO groups (group_id, name, creator_fp, epoch, left_group, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT (group_id) DO UPDATE SET name = excluded.name, epoch = excluded.epoch,
                 left_group = excluded.left_group",
            params![g.group_id, g.name, g.creator_fp, g.epoch, g.left, now],
        )?;
        tx.execute(
            "DELETE FROM group_members WHERE group_id = ?1",
            [g.group_id],
        )?;
        for (position, (fp, name)) in g.members.iter().enumerate() {
            tx.execute(
                "INSERT INTO group_members (group_id, fingerprint, name, position) VALUES (?1, ?2, ?3, ?4)",
                params![g.group_id, fp, name, i64::try_from(position).unwrap_or(i64::MAX)],
            )?;
        }
        tx.commit()?;
        Ok(GroupApply::Applied)
    }

    pub fn set_group_left(&mut self, group_id: &[u8; 16]) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE groups SET left_group = 1 WHERE group_id = ?1",
            [group_id],
        )?;
        Ok(())
    }

    /// Returns false if the message is already stored.
    pub fn insert_group_message(&mut self, m: &GroupMessageRow) -> Result<bool, DbError> {
        Ok(self.conn.execute(
            "INSERT OR IGNORE INTO group_messages (group_msg_id, group_id, author_fp, outgoing, body, reply_to, created_at, sent_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![m.group_msg_id, m.group_id, m.author_fp, m.outgoing, m.body, m.reply_to, m.created_at, m.sent_at_ms],
        )? > 0)
    }

    /// Newest first.
    /// The group a group message belongs to, if it is stored here.
    pub fn group_of_message(&self, group_msg_id: &[u8; 16]) -> Result<Option<[u8; 16]>, DbError> {
        let id: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT group_id FROM group_messages WHERE group_msg_id = ?1",
                [group_msg_id],
                |r| r.get(0),
            )
            .optional()?;
        id.map(|v| blob16(&v)).transpose()
    }

    pub fn group_messages(
        &self,
        group_id: &[u8; 16],
        limit: u32,
    ) -> Result<Vec<GroupMessageRow>, DbError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT group_msg_id, author_fp, outgoing, body, reply_to, created_at, sent_at_ms
             FROM group_messages WHERE group_id = ?1 ORDER BY created_at DESC, sent_at_ms DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![group_id, limit], |r| {
            Ok((
                r.get::<_, Vec<u8>>(0)?,
                r.get::<_, Vec<u8>>(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get::<_, Option<Vec<u8>>>(4)?,
                r.get(5)?,
                r.get(6)?,
            ))
        })?;
        rows.map(|r| {
            let (id, fp, outgoing, body, reply, created_at, sent_at_ms) = r?;
            Ok(GroupMessageRow {
                group_msg_id: blob16(&id)?,
                group_id: *group_id,
                author_fp: blob32(&fp)?,
                outgoing,
                body,
                reply_to: opt16(reply)?,
                created_at,
                sent_at_ms,
            })
        })
        .collect()
    }

    /// Holds a group message until its group state allows a decision. Bounded
    /// per contact by [`MAX_GROUP_PENDING_PER_CONTACT`].
    pub fn hold_group_message(
        &mut self,
        p: &PendingGroupMessage,
        now: i64,
    ) -> Result<bool, DbError> {
        let held: i64 = self.conn.query_row(
            "SELECT count(*) FROM group_pending WHERE contact_id = ?1",
            [p.contact_id],
            |r| r.get(0),
        )?;
        if held >= MAX_GROUP_PENDING_PER_CONTACT {
            return Ok(false);
        }
        let m = &p.message;
        Ok(self.conn.execute(
            "INSERT OR IGNORE INTO group_pending (group_msg_id, contact_id, group_id, epoch, reply_to, body, sent_at_ms, received_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![m.group_msg_id, p.contact_id, m.group_id, p.epoch, m.reply_to, m.body, m.sent_at_ms, now],
        )? > 0)
    }

    /// Removes and returns every held message for a group.
    pub fn take_group_pending(
        &mut self,
        group_id: &[u8; 16],
    ) -> Result<Vec<PendingGroupMessage>, DbError> {
        let tx = self.conn.transaction()?;
        let rows: Vec<PendingGroupMessage> = {
            let mut stmt = tx.prepare(
                "SELECT group_msg_id, contact_id, epoch, reply_to, body, sent_at_ms, received_at
                 FROM group_pending WHERE group_id = ?1",
            )?;
            let rows = stmt.query_map([group_id], |r| {
                Ok((
                    r.get::<_, Vec<u8>>(0)?,
                    r.get::<_, Vec<u8>>(1)?,
                    r.get(2)?,
                    r.get::<_, Option<Vec<u8>>>(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                ))
            })?;
            rows.map(|r| {
                let (id, contact, epoch, reply, body, sent_at_ms, received_at) = r?;
                Ok(PendingGroupMessage {
                    contact_id: blob16(&contact)?,
                    epoch,
                    message: GroupMessageRow {
                        group_msg_id: blob16(&id)?,
                        group_id: *group_id,
                        author_fp: [0; 32],
                        outgoing: false,
                        body,
                        reply_to: opt16(reply)?,
                        created_at: received_at,
                        sent_at_ms,
                    },
                })
            })
            .collect::<Result<Vec<_>, DbError>>()?
        };
        tx.execute("DELETE FROM group_pending WHERE group_id = ?1", [group_id])?;
        tx.commit()?;
        Ok(rows)
    }

    pub fn add_group_copy(
        &mut self,
        copy_id: &[u8; 16],
        group_msg_id: &[u8; 16],
        contact_id: &[u8; 16],
        envelope_id: &[u8; 16],
        ack_secret: &[u8; 16],
        expiry_min: u32,
    ) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO group_copies (copy_id, group_msg_id, contact_id, envelope_id, ack_secret, expiry_min)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![copy_id, group_msg_id, contact_id, envelope_id, ack_secret, expiry_min],
        )?;
        Ok(())
    }

    /// Marks one pairwise copy delivered, by copy (inner message) id or envelope
    /// id. Returns the copy's `(ack_secret, expiry_min)` if it changed state.
    pub fn mark_group_copy_delivered(
        &mut self,
        id: &[u8; 16],
        contact_id: Option<&[u8; 16]>,
    ) -> Result<Option<([u8; 16], u32)>, DbError> {
        let row: Option<(Vec<u8>, u32)> = self
            .conn
            .query_row(
                "UPDATE group_copies SET delivered = 1
                 WHERE (copy_id = ?1 OR envelope_id = ?1) AND (?2 IS NULL OR contact_id = ?2) AND NOT delivered
                 RETURNING ack_secret, expiry_min",
                params![id, contact_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        row.map(|(secret, expiry)| Ok((blob16(&secret)?, expiry)))
            .transpose()
    }

    /// `(delivered, sent)` pairwise copies of one of my group messages.
    pub fn group_delivery(&self, group_msg_id: &[u8; 16]) -> Result<(u32, u32), DbError> {
        Ok(self.conn.query_row(
            "SELECT coalesce(sum(delivered), 0), count(*) FROM group_copies WHERE group_msg_id = ?1",
            [group_msg_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?)
    }

    /// An event is stored if its target is in the same conversation, or (target
    /// not yet arrived) while the contact is under [`MAX_ORPHANS_PER_CONTACT`].
    fn event_admissible(&self, ev: &EventMeta) -> Result<bool, DbError> {
        let target_contact: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT contact_id FROM messages WHERE message_id = ?1",
                [ev.target],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(c) = target_contact {
            return Ok(c == ev.contact_id);
        }
        let orphans: i64 = self.conn.query_row(
            "SELECT (SELECT count(*) FROM reactions WHERE contact_id = ?1 AND target NOT IN (SELECT message_id FROM messages))
                  + (SELECT count(*) FROM edits WHERE contact_id = ?1 AND target NOT IN (SELECT message_id FROM messages))
                  + (SELECT count(*) FROM tombstones WHERE contact_id = ?1 AND NOT local AND target NOT IN (SELECT message_id FROM messages))",
            [ev.contact_id],
            |r| r.get(0),
        )?;
        Ok(orphans < MAX_ORPHANS_PER_CONTACT)
    }
}

type RowTuple = (
    Vec<u8>,
    Vec<u8>,
    bool,
    String,
    i64,
    i64,
    Option<Vec<u8>>,
    Option<Vec<u8>>,
    Option<u32>,
    i64,
    Option<Vec<u8>>,
    bool,
    bool,
    bool,
    u32,
    Option<i64>,
    Option<i64>,
    bool,
    bool,
);

fn row_tuple(r: &rusqlite::Row<'_>) -> rusqlite::Result<RowTuple> {
    Ok((
        r.get(0)?,
        r.get(1)?,
        r.get(2)?,
        r.get(3)?,
        r.get(4)?,
        r.get(5)?,
        r.get(6)?,
        r.get(7)?,
        r.get(8)?,
        r.get(9)?,
        r.get(10)?,
        r.get(11)?,
        r.get(12)?,
        r.get(13)?,
        r.get(14)?,
        r.get(15)?,
        r.get(16)?,
        r.get(17)?,
        r.get(18)?,
    ))
}

fn message_from(t: RowTuple) -> Result<MessageRow, DbError> {
    let (
        mid,
        cid,
        outgoing,
        body,
        created_at,
        state,
        env,
        ack,
        expiry_min,
        sent_at_ms,
        reply_to,
        forwarded,
        deleted,
        edited,
        timer_secs,
        read_at,
        expire_at,
        pinned,
        starred,
    ) = t;
    Ok(MessageRow {
        message_id: blob16(&mid)?,
        contact_id: blob16(&cid)?,
        outgoing,
        body,
        created_at,
        state: MessageState::from_i64(state)?,
        envelope_id: opt16(env)?,
        ack_secret: opt16(ack)?,
        expiry_min,
        sent_at_ms,
        reply_to: opt16(reply_to)?,
        forwarded,
        deleted,
        edited,
        timer_secs,
        read_at,
        expire_at,
        pinned,
        starred,
    })
}

// ponytail: FTS rowid is the first 8 bytes of the random 128-bit message id; a
// collision (~n²/2⁶⁴) only drops one message from search. Map ids through a
// rowid table if search must be exact.
fn fts_key(message_id: &[u8; 16]) -> i64 {
    let [a, b, c, d, e, f, g, h, ..] = *message_id;
    i64::from_be_bytes([a, b, c, d, e, f, g, h])
}

/// Re-derives the search entry for one message from its effective text.
fn reindex(tx: &rusqlite::Transaction<'_>, message_id: &[u8; 16]) -> Result<(), DbError> {
    let key = fts_key(message_id);
    tx.execute("DELETE FROM messages_fts WHERE rowid = ?1", [key])?;
    let sql = format!("{SELECT_MESSAGE} WHERE m.message_id = ?1");
    if let Some(m) = tx
        .query_row(&sql, [message_id], row_tuple)
        .optional()?
        .map(message_from)
        .transpose()?
        && !m.deleted
        && !m.body.is_empty()
    {
        tx.execute(
            "INSERT INTO messages_fts (rowid, body, message_id, contact_id) VALUES (?1, ?2, ?3, ?4)",
            params![key, m.body, m.message_id, m.contact_id],
        )?;
    }
    Ok(())
}

fn purge_annotations(tx: &rusqlite::Transaction<'_>, message_id: &[u8; 16]) -> Result<(), DbError> {
    tx.execute("DELETE FROM locations WHERE message_id = ?1", [message_id])?;
    tx.execute("DELETE FROM polls WHERE poll_id = ?1", [message_id])?;
    tx.execute("DELETE FROM poll_votes WHERE poll_id = ?1", [message_id])?;
    tx.execute(
        "DELETE FROM attachment_chunks WHERE (contact_id, attachment_id) IN
             (SELECT contact_id, attachment_id FROM attachments WHERE message_id = ?1)",
        [message_id],
    )?;
    tx.execute(
        "DELETE FROM attachments WHERE message_id = ?1",
        [message_id],
    )?;
    tx.execute("DELETE FROM reactions WHERE target = ?1", [message_id])?;
    tx.execute("DELETE FROM edits WHERE target = ?1", [message_id])?;
    tx.execute(
        "UPDATE messages SET pinned = 0, starred = 0 WHERE message_id = ?1",
        [message_id],
    )?;
    Ok(())
}

fn purge_message(tx: &rusqlite::Transaction<'_>, message_id: &[u8; 16]) -> Result<(), DbError> {
    purge_annotations(tx, message_id)?;
    tx.execute(
        "DELETE FROM messages_fts WHERE rowid = ?1",
        [fts_key(message_id)],
    )?;
    tx.execute("DELETE FROM messages WHERE message_id = ?1", [message_id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_1_database_migrates_with_data_intact() {
        let dir = std::env::temp_dir().join(format!("vl-migrate-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("c.db");
        let key = [3u8; 32];
        {
            let conn = open_encrypted(&path, &key, MIGRATIONS.get(..1).unwrap()).unwrap();
            conn.execute_batch(
                "INSERT INTO contacts (contact_id, contact_ref, display_name, card, paired_at)
                 VALUES (x'01010101010101010101010101010101', x'02020202020202020202020202020202', 'C', x'', 1);
                 INSERT INTO messages (message_id, contact_id, outgoing, body, created_at, state)
                 VALUES (x'03030303030303030303030303030303', x'01010101010101010101010101010101', 1, 'kept', 5, 0);",
            )
            .unwrap();
        }
        let db = ConversationDb::open(&path, &key).unwrap();
        let m = db.message(&[3; 16]).unwrap().unwrap();
        assert_eq!(
            (m.body.as_str(), m.deleted, m.timer_secs),
            ("kept", false, 0)
        );
        assert!(!db.contacts().unwrap()[0].blocked);
        drop(db);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
