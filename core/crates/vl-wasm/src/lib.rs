//! The web chat's node (ADR-040). The browser runs the same Rust core as the
//! phones, through one JSON call: `call(method, now_secs, args)`, where `args`
//! is a JSON array and the result is JSON (byte strings are arrays of numbers).
//! Method names match the app's host API. Errors are the host error codes
//! (`TextTooLong`, `Storage: …`), as on iPhone and Android.
//!
//! In the browser the databases live in SQLite's in-memory file system; the
//! web host loads their bytes at start and saves them after every change.

use serde::Serialize;
use serde_json::Value;
use vl_ffi::{VlError, VlNode};

pub const RELAY_DB: &str = "relay.db";
pub const CONVERSATION_DB: &str = "conversation.db";

/// Turns a core error into the code the app understands.
fn code(e: VlError) -> String {
    match e {
        VlError::InvalidInput { reason } => format!("InvalidInput: {reason}"),
        VlError::Storage { reason } => format!("Storage: {reason}"),
        VlError::Crypto { reason } => format!("Crypto: {reason}"),
        other => format!("{other:?}"),
    }
}

/// Positional JSON arguments.
struct Args(Vec<Value>);

impl Args {
    fn get(&self, i: usize) -> Result<&Value, String> {
        self.0
            .get(i)
            .ok_or_else(|| format!("InvalidInput: missing argument {i}"))
    }
    fn bytes(&self, i: usize) -> Result<Vec<u8>, String> {
        bytes(self.get(i)?)
    }
    fn opt_bytes(&self, i: usize) -> Result<Option<Vec<u8>>, String> {
        match self.get(i)? {
            Value::Null => Ok(None),
            v => bytes(v).map(Some),
        }
    }
    fn list_bytes(&self, i: usize) -> Result<Vec<Vec<u8>>, String> {
        self.get(i)?
            .as_array()
            .ok_or("InvalidInput: expected a list")?
            .iter()
            .map(bytes)
            .collect()
    }
    fn string(&self, i: usize) -> Result<String, String> {
        Ok(self
            .get(i)?
            .as_str()
            .ok_or("InvalidInput: expected text")?
            .to_owned())
    }
    fn opt_string(&self, i: usize) -> Result<Option<String>, String> {
        match self.get(i)? {
            Value::Null => Ok(None),
            _ => self.string(i).map(Some),
        }
    }
    fn strings(&self, i: usize) -> Result<Vec<String>, String> {
        serde_json::from_value(self.get(i)?.clone())
            .map_err(|_| "InvalidInput: expected texts".into())
    }
    fn bool(&self, i: usize) -> Result<bool, String> {
        self.get(i)?
            .as_bool()
            .ok_or_else(|| "InvalidInput: expected true or false".into())
    }
    fn int(&self, i: usize) -> Result<i64, String> {
        self.get(i)?
            .as_i64()
            .ok_or_else(|| "InvalidInput: expected a number".into())
    }
    /// A number clamped into `T` (like the phone hosts' coerceIn).
    fn num<T: TryFrom<i64> + Bounded>(&self, i: usize) -> Result<T, String> {
        let v = self.int(i)?.clamp(T::MIN, T::MAX);
        T::try_from(v).map_err(|_| "InvalidInput: number out of range".into())
    }
}

trait Bounded {
    const MIN: i64;
    const MAX: i64;
}
macro_rules! bounded {
    ($($t:ty),*) => {$(impl Bounded for $t {
        const MIN: i64 = <$t>::MIN as i64;
        const MAX: i64 = if (<$t>::MAX as u64) > i64::MAX as u64 { i64::MAX } else { <$t>::MAX as i64 };
    })*};
}
bounded!(u8, u16, u32, u64, i32);

fn bytes(v: &Value) -> Result<Vec<u8>, String> {
    serde_json::from_value(v.clone()).map_err(|_| "InvalidInput: expected bytes".into())
}

fn json<T: Serialize>(v: T) -> Result<String, String> {
    serde_json::to_string(&v).map_err(|e| format!("Storage: {e}"))
}

/// One web chat node.
pub struct Web {
    node: Option<VlNode>,
}

impl std::fmt::Debug for Web {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Web")
    }
}

impl Web {
    /// Opens the node on database files at these paths (in the browser,
    /// files in SQLite's memory file system). Keys are 32 bytes each.
    pub fn open(
        relay_db: &str,
        conversation_db: &str,
        relay_key: Vec<u8>,
        conversation_key: Vec<u8>,
    ) -> Result<Self, String> {
        let node = VlNode::open(
            relay_db.into(),
            relay_key,
            conversation_db.into(),
            conversation_key,
        )
        .map_err(code)?;
        Ok(Self { node: Some(node) })
    }

    /// Drops the node; the web host then deletes the stored bytes and keys.
    pub fn close(&mut self) {
        self.node = None;
    }

    /// Calls one host-API method. See the module docs.
    pub fn call(&self, method: &str, now: u64, args: &str) -> Result<String, String> {
        let a = Args(
            serde_json::from_str::<Vec<Value>>(args)
                .map_err(|_| "InvalidInput: arguments must be a JSON array")?,
        );
        match method {
            "maxTextBytes" => return json(vl_ffi::max_text_bytes()),
            "reactionSet" => return json(vl_ffi::reaction_set()),
            "qrCode" => return json(vl_ffi::qr_matrix(a.string(0)?).map_err(code)?),
            _ => {}
        }
        let n = self.node.as_ref().ok_or("Wiped")?;
        let r = |e: VlError| code(e);
        match method {
            // Lifecycle and identity
            "hasIdentity" => json(n.has_identity()),
            "createIdentity" => json(n.create_identity(now).map_err(r)?),
            "myFingerprint" => json(n.my_fingerprint().map_err(r)?),
            "myIdCode" => json(n.my_id_code().map_err(r)?),
            "setting" => json(n.setting(a.string(0)?).map_err(r)?),
            "setSetting" => json(n.set_setting(a.string(0)?, a.opt_string(1)?).map_err(r)?),
            "maintain" => json(n.maintain(now).map_err(r)?),
            // Internet mailbox
            "internetOutbox" => json(n.internet_outbox(now).map_err(r)?),
            "internetUploaded" => json(n.internet_uploaded(a.list_bytes(0)?, now).map_err(r)?),
            "internetTags" => json(n.internet_tags().map_err(r)?),
            "receiveInternet" => json(n.receive_internet(a.bytes(0)?, now).map_err(r)?),
            // Pairing and contacts
            "pairingCode" => json(n.pairing_code(a.string(0)?, now).map_err(r)?),
            "completePairing" => json(n.complete_pairing_code(a.string(0)?, now).map_err(r)?),
            "contacts" => json(n.contacts().map_err(r)?),
            "setVerified" => json(n.set_verified(a.bytes(0)?, a.bool(1)?).map_err(r)?),
            "setBlocked" => json(n.set_blocked(a.bytes(0)?, a.bool(1)?, now).map_err(r)?),
            "renameContact" => json(n.rename_contact(a.bytes(0)?, a.string(1)?).map_err(r)?),
            "deleteContact" => json(n.delete_contact(a.bytes(0)?).map_err(r)?),
            // Conversations
            "conversationSummaries" => json(n.conversation_summaries().map_err(r)?),
            "conversation" => json(n.conversation(a.bytes(0)?, a.num(1)?).map_err(r)?),
            "message" => json(n.message(a.bytes(0)?).map_err(r)?),
            "sendMessage" => json(
                n.send_message(a.bytes(0)?, a.string(1)?, a.opt_bytes(2)?, a.bool(3)?, now)
                    .map_err(r)?,
            ),
            "react" => json(
                n.react(a.bytes(0)?, a.bytes(1)?, a.num(2)?, now)
                    .map_err(r)?,
            ),
            "reactions" => json(n.reactions(a.bytes(0)?).map_err(r)?),
            "editMessage" => json(
                n.edit_message(a.bytes(0)?, a.bytes(1)?, a.string(2)?, now)
                    .map_err(r)?,
            ),
            "deleteForEveryone" => json(
                n.delete_for_everyone(a.bytes(0)?, a.bytes(1)?, now)
                    .map_err(r)?,
            ),
            "deleteForMe" => json(n.delete_for_me(a.bytes(0)?, now).map_err(r)?),
            "setDisappearingTimer" => json(
                n.set_disappearing_timer(a.bytes(0)?, a.num(1)?, now)
                    .map_err(r)?,
            ),
            "disappearingTimer" => json(n.disappearing_timer(a.bytes(0)?).map_err(r)?),
            "markRead" => json(n.mark_read(a.bytes(0)?, now).map_err(r)?),
            "search" => json(
                n.search(a.string(0)?, a.opt_bytes(1)?, a.num(2)?)
                    .map_err(r)?,
            ),
            "saveDraft" => json(n.save_draft(a.bytes(0)?, a.string(1)?, now).map_err(r)?),
            "draft" => json(n.draft(a.bytes(0)?).map_err(r)?),
            "setPinned" => json(n.set_pinned(a.bytes(0)?, a.bool(1)?).map_err(r)?),
            "setStarred" => json(n.set_starred(a.bytes(0)?, a.bool(1)?).map_err(r)?),
            "flagged" => json(n.flagged(a.opt_bytes(0)?, a.bool(1)?).map_err(r)?),
            "pendingOutgoing" => json(n.pending_outgoing().map_err(r)?),
            "messageEvents" => json(n.message_events(a.bytes(0)?).map_err(r)?),
            "resend" => json(n.resend(a.bytes(0)?, now).map_err(r)?),
            // Attachments
            "sendAttachment" => json(
                n.send_attachment(
                    a.bytes(0)?,
                    a.num(1)?,
                    a.string(2)?,
                    a.bytes(3)?,
                    a.string(4)?,
                    a.opt_bytes(5)?,
                    now,
                )
                .map_err(r)?,
            ),
            "attachmentData" => json(n.attachment_data(a.bytes(0)?).map_err(r)?),
            "setCarryCap" => json(n.set_carry_cap(a.num(0)?).map_err(r)?),
            "carryCap" => json(n.carry_cap().map_err(r)?),
            // Groups
            "createGroup" => json(
                n.create_group(a.string(0)?, a.string(1)?, a.list_bytes(2)?, now)
                    .map_err(r)?,
            ),
            "setGroupMembers" => json(
                n.set_group_members(a.bytes(0)?, a.string(1)?, a.list_bytes(2)?, now)
                    .map_err(r)?,
            ),
            "leaveGroup" => json(n.leave_group(a.bytes(0)?, now).map_err(r)?),
            "groups" => json(n.groups().map_err(r)?),
            "groupMessages" => json(n.group_messages(a.bytes(0)?, a.num(1)?).map_err(r)?),
            "sendGroupMessage" => json(
                n.send_group_message(a.bytes(0)?, a.string(1)?, a.opt_bytes(2)?, now)
                    .map_err(r)?,
            ),
            "groupDelivery" => json(n.group_delivery(a.bytes(0)?).map_err(r)?),
            // Sharing (ADR-034..037)
            "sendLocation" => json(
                n.send_location(a.bytes(0)?, a.num(1)?, a.num(2)?, a.num(3)?, a.num(4)?, now)
                    .map_err(r)?,
            ),
            "sendPoll" => json(
                n.send_poll(a.bytes(0)?, a.string(1)?, a.strings(2)?, a.bool(3)?, now)
                    .map_err(r)?,
            ),
            "sendGroupPoll" => json(
                n.send_group_poll(a.bytes(0)?, a.string(1)?, a.strings(2)?, a.bool(3)?, now)
                    .map_err(r)?,
            ),
            "vote" => json(n.vote(a.bytes(0)?, a.num(1)?, now).map_err(r)?),
            "introduce" => json(n.introduce(a.bytes(0)?, a.bytes(1)?, now).map_err(r)?),
            "acceptIntro" => json(n.accept_intro(a.bytes(0)?, a.string(1)?, now).map_err(r)?),
            "declineIntro" => json(n.decline_intro(a.bytes(0)?).map_err(r)?),
            "paperMessage" => json(n.paper_message(a.bytes(0)?).map_err(r)?),
            "receivePaper" => json(n.receive_paper(a.string(0)?, now).map_err(r)?),
            // Relay settings that still apply on the web
            "resyncSecs" => json(n.resync_secs()),
            "setResyncSecs" => json(n.set_resync_secs(a.num(0)?).map_err(r)?),
            "stats" => json(n.stats().map_err(r)?),
            other => Err(format!("InvalidInput: unknown method {other}")),
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod browser {
    use super::{CONVERSATION_DB, RELAY_DB, Web};
    use sqlite_wasm_rs::{MemVfsUtil, WasmOsCallback};
    use wasm_bindgen::prelude::*;

    fn vfs() -> MemVfsUtil<WasmOsCallback> {
        MemVfsUtil::new()
    }

    /// The browser-facing node. Bytes in and out are `Uint8Array`s.
    #[wasm_bindgen]
    #[derive(Debug)]
    pub struct WebNode(Web);

    #[wasm_bindgen]
    impl WebNode {
        /// Opens the node from previously saved database bytes (empty = new).
        #[wasm_bindgen(constructor)]
        pub fn new(
            relay: Vec<u8>,
            conversation: Vec<u8>,
            relay_key: Vec<u8>,
            conversation_key: Vec<u8>,
        ) -> Result<WebNode, JsError> {
            let v = vfs();
            for (name, bytes) in [(RELAY_DB, &relay), (CONVERSATION_DB, &conversation)] {
                v.delete_db(name);
                if !bytes.is_empty() {
                    v.import_db(name, bytes)
                        .map_err(|e| JsError::new(&format!("Storage: {e}")))?;
                }
            }
            Web::open(RELAY_DB, CONVERSATION_DB, relay_key, conversation_key)
                .map(WebNode)
                .map_err(|e| JsError::new(&e))
        }

        /// Returns `{"ok": result}` or `{"err": "Code: reason"}`; it never
        /// throws, because thrown messages do not reach Dart reliably.
        pub fn call(&self, method: &str, now: f64, args: &str) -> String {
            // ponytail: f64 seconds from Date.now(); exact until the year 285 million.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let now = now.max(0.0) as u64;
            match self.0.call(method, now, args) {
                Ok(out) => format!("{{\"ok\":{out}}}"),
                Err(e) => serde_json::json!({ "err": e }).to_string(),
            }
        }

        /// Current bytes of one database, to save in browser storage.
        pub fn export(&self, relay: bool) -> Result<Vec<u8>, JsError> {
            vfs()
                .export_db(if relay { RELAY_DB } else { CONVERSATION_DB })
                .map_err(|e| JsError::new(&format!("Storage: {e}")))
        }

        /// Panic wipe: closes the node and erases both databases from memory.
        pub fn wipe(&mut self) {
            self.0.close();
            let v = vfs();
            v.delete_db(RELAY_DB);
            v.delete_db(CONVERSATION_DB);
        }
    }
}
