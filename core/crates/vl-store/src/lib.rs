//! Encrypted persistence (ADR-010, ADR-018). Two SQLCipher databases with
//! independent keys supplied by the platform key store:
//! - [`RelayDb`] (Class R): ciphertext envelopes, seen/vaccine sets, pairwise
//!   keys and tags. Must open while the device is locked after first unlock.
//! - [`ConversationDb`] (Class C): identity, contacts, Olm sessions, plaintext
//!   messages. Opened only while unlocked.

mod conversation;
mod relay;

pub use conversation::{
    AttachmentRow, Contact, ConversationDb, DELETE_WINDOW_MS, EDIT_WINDOW_MS, EventMeta,
    GroupApply, GroupMessageRow, GroupRow, MAX_GROUP_PENDING_PER_CONTACT, MAX_ORPHANS_PER_CONTACT,
    MAX_PARTIAL_ATTACHMENT_BYTES, MessageRow, MessageState, ORPHAN_TTL_SECS, PendingGroupMessage,
    Reaction,
};
pub use relay::RelayDb;

use rusqlite::Connection;
use std::fmt;
use std::path::Path;
use zeroize::Zeroizing;

#[derive(Debug)]
pub enum DbError {
    /// Wrong key or not a database: SQLCipher cannot authenticate the file.
    WrongKeyOrCorrupt,
    /// Schema written by a newer app version.
    TooNew(i64),
    Sqlite(rusqlite::Error),
    Corrupt(&'static str),
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongKeyOrCorrupt => f.write_str("database key is wrong or the file is corrupt"),
            Self::TooNew(v) => write!(f, "database schema {v} is newer than this app"),
            Self::Sqlite(e) => write!(f, "sqlite: {e}"),
            Self::Corrupt(what) => write!(f, "corrupt record: {what}"),
        }
    }
}

impl std::error::Error for DbError {}

impl From<rusqlite::Error> for DbError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e)
    }
}

/// Opens (or creates) a SQLCipher database with a raw 256-bit key and applies
/// forward-only migrations. `path` may be `":memory:"` for tests.
pub(crate) fn open_encrypted(
    path: &Path,
    key: &[u8; 32],
    migrations: &[&str],
) -> Result<Connection, DbError> {
    let conn = Connection::open(path)?;
    // SQLCipher's default WARN-level logging goes to the public device log
    // (os_log/logcat) and records key/HMAC failures; diagnostics must be counters only.
    conn.execute_batch("PRAGMA cipher_log_level = NONE;")?;
    let hex: Zeroizing<String> = Zeroizing::new(key.iter().map(|b| format!("{b:02x}")).collect());
    let pragma = Zeroizing::new(format!("PRAGMA key = \"x'{}'\";", hex.as_str()));
    conn.execute_batch(&pragma)?;
    if conn
        .query_row("SELECT count(*) FROM sqlite_master", [], |r| {
            r.get::<_, i64>(0)
        })
        .is_err()
    {
        return Err(DbError::WrongKeyOrCorrupt);
    }
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = FULL;
         PRAGMA foreign_keys = ON;
         PRAGMA secure_delete = ON;",
    )?;
    migrate(&conn, migrations)?;
    Ok(conn)
}

fn migrate(conn: &Connection, migrations: &[&str]) -> Result<(), DbError> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let target = i64::try_from(migrations.len()).unwrap_or(i64::MAX);
    if current > target {
        return Err(DbError::TooNew(current));
    }
    for (i, sql) in migrations
        .iter()
        .enumerate()
        .skip(usize::try_from(current).unwrap_or(usize::MAX))
    {
        let version = i + 1;
        conn.execute_batch(&format!(
            "BEGIN IMMEDIATE; {sql} PRAGMA user_version = {version}; COMMIT;"
        ))?;
    }
    Ok(())
}

pub(crate) fn blob16(bytes: &[u8]) -> Result<[u8; 16], DbError> {
    bytes.try_into().map_err(|_| DbError::Corrupt("16-byte id"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlcipher_device_logging_is_off() {
        let conn = open_encrypted(Path::new(":memory:"), &[7; 32], &[]).unwrap();
        let level: String = conn
            .query_row("PRAGMA cipher_log_level", [], |r| r.get(0))
            .unwrap();
        assert_eq!(level, "NONE");
    }
}
