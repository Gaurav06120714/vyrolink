//! Conversation content carried inside the Olm plaintext of a `CONTENT` (0x11)
//! inner message (Doc 21 §4.1). Every change to a message is a new event that
//! references its target by message id, so events can arrive in any order.

use crate::{MAX_NAME_LEN, ProtoError, Reader};

pub const CONTENT_VERSION: u8 = 1;
/// Fixed reaction set (index 1..=8 on the wire; 0 removes a reaction).
pub const REACTIONS: [&str; 8] = ["👍", "👎", "😀", "😂", "😢", "❤️", "🚀", "✅"];
pub const MAX_EDIT_SEQ: u8 = 10;
pub const MIN_TIMER_SECS: u32 = 60;
pub const MAX_TIMER_SECS: u32 = 365 * 24 * 60 * 60;
/// Small groups only (ADR-024): every message is one pairwise copy per member.
pub const MAX_GROUP_MEMBERS: usize = 8;
/// Attachments (ADR-026): `kind` values, size caps and chunking.
pub const ATTACHMENT_IMAGE: u8 = 1;
pub const ATTACHMENT_FILE: u8 = 2;
pub const MAX_IMAGE_BYTES: u32 = 16 * 1024;
pub const MAX_FILE_BYTES: u32 = 64 * 1024;
/// Plaintext bytes per chunk; a sealed chunk inner message (44 + 20 + 1,416)
/// fits the 2,048 bucket.
pub const CHUNK_BYTES: usize = 1400;
pub const MAX_ATTACHMENT_NAME: usize = 64;
pub const MAX_CAPTION_BYTES: usize = 1024;
/// Poll limits (ADR-035).
pub const MAX_POLL_OPTIONS: usize = 10;
pub const MAX_POLL_QUESTION_BYTES: usize = 300;
pub const MAX_POLL_OPTION_BYTES: usize = 100;
const FLAG_GROUP: u8 = 0x04;
const FLAG_MULTI: u8 = 0x08;

/// Number of chunks an attachment of `total_len` bytes is split into.
pub const fn chunk_count_for(total_len: u32) -> u32 {
    total_len.div_ceil(CHUNK_BYTES as u32)
}

const CT_TEXT: u8 = 0x01;
const CT_REACTION: u8 = 0x02;
const CT_EDIT: u8 = 0x03;
const CT_DELETE: u8 = 0x04;
const CT_TIMER: u8 = 0x05;
const CT_GROUP_STATE: u8 = 0x06;
const CT_GROUP_TEXT: u8 = 0x07;
const CT_GROUP_LEAVE: u8 = 0x08;
const CT_ATTACHMENT: u8 = 0x09;
const CT_LOCATION: u8 = 0x0A;
const CT_POLL: u8 = 0x0B;
const CT_POLL_VOTE: u8 = 0x0C;
const CT_INTRO_ASK: u8 = 0x0D;
const CT_INTRO_CARD: u8 = 0x0E;
const CT_INTRO_DELIVER: u8 = 0x0F;
/// Largest pairing card carried in an introduction (ADR-037).
pub const MAX_INTRO_CARD_BYTES: usize = 1024;

const FLAG_REPLY: u8 = 0x01;
const FLAG_FORWARDED: u8 = 0x02;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    Text {
        reply_to: Option<[u8; 16]>,
        forwarded: bool,
        /// Disappearing-message timer in force for the conversation; 0 = off.
        timer_secs: u32,
        text: String,
    },
    /// `reaction` 0 removes the sender's reaction; 1..=8 index [`REACTIONS`].
    Reaction {
        target: [u8; 16],
        reaction: u8,
    },
    Edit {
        target: [u8; 16],
        seq: u8,
        text: String,
    },
    Delete {
        target: [u8; 16],
    },
    Timer {
        timer_secs: u32,
    },
    /// The group's full roster at `epoch`. Only the group's creator may send it;
    /// members are identified by their pairing-card Ed25519 fingerprint keys.
    GroupState {
        group_id: [u8; 16],
        epoch: u32,
        name: String,
        members: Vec<GroupMember>,
    },
    /// One pairwise copy of a group message; `group_msg_id` is shared by all copies.
    GroupText {
        group_id: [u8; 16],
        group_msg_id: [u8; 16],
        epoch: u32,
        reply_to: Option<[u8; 16]>,
        text: String,
    },
    /// Sent to the creator by a member who leaves.
    GroupLeave {
        group_id: [u8; 16],
    },
    /// Announces an attachment (ADR-026); it is shown like a message with
    /// `caption`. Its bytes follow as `AttachmentChunk` inner messages
    /// encrypted with `key`, in any order and over any number of encounters.
    Attachment {
        attachment_id: [u8; 16],
        kind: u8,
        total_len: u32,
        sha256: [u8; 32],
        /// One-time ChaCha20-Poly1305 key for this attachment's chunks.
        key: [u8; 32],
        reply_to: Option<[u8; 16]>,
        name: String,
        caption: String,
    },
    /// A one-off position shared on purpose by the sender (ADR-034).
    /// Degrees × 10⁷ (WGS 84), accuracy in metres, time of the GPS fix.
    Location {
        lat_e7: i32,
        lon_e7: i32,
        accuracy_m: u16,
        captured_ms: u64,
    },
    /// A poll (ADR-035). `poll_id` is the message id it is shown under: the
    /// inner message id in a chat, the shared `group_msg_id` in a group.
    Poll {
        group: Option<GroupRef>,
        poll_id: [u8; 16],
        question: String,
        options: Vec<String>,
        multi: bool,
    },
    /// The sender's whole current choice for a poll (bit i = option i); 0 withdraws.
    PollVote {
        group: Option<GroupRef>,
        poll_id: [u8; 16],
        choices: u16,
    },
    /// Introductions (ADR-037). The introducer asks each side; each side that
    /// accepts sends the introducer a fresh signed card; the introducer then
    /// hands each side the other's card. `other_fp` names who the card must be.
    IntroAsk {
        intro_id: [u8; 16],
        other_fp: [u8; 32],
        other_name: String,
    },
    IntroCard {
        intro_id: [u8; 16],
        card: Vec<u8>,
    },
    IntroDeliver {
        intro_id: [u8; 16],
        card: Vec<u8>,
    },
}

/// Group a poll or vote belongs to, and the roster epoch it was sent under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupRef {
    pub group_id: [u8; 16],
    pub epoch: u32,
}

fn check_location(lat_e7: i32, lon_e7: i32) -> Result<(), ProtoError> {
    if !(-900_000_000..=900_000_000).contains(&lat_e7)
        || !(-1_800_000_000..=1_800_000_000).contains(&lon_e7)
    {
        return Err(ProtoError::InvalidField("location"));
    }
    Ok(())
}

fn check_poll(question: &str, options: &[String]) -> Result<(), ProtoError> {
    if question.trim().is_empty()
        || question.len() > MAX_POLL_QUESTION_BYTES
        || !(2..=MAX_POLL_OPTIONS).contains(&options.len())
        || options
            .iter()
            .any(|o| o.trim().is_empty() || o.len() > MAX_POLL_OPTION_BYTES)
    {
        return Err(ProtoError::InvalidField("poll"));
    }
    Ok(())
}

fn put_group(out: &mut Vec<u8>, group: Option<&GroupRef>) {
    if let Some(g) = group {
        out.extend_from_slice(&g.group_id);
        out.extend_from_slice(&g.epoch.to_be_bytes());
    }
}

fn get_group(r: &mut Reader<'_>, flags: u8) -> Result<Option<GroupRef>, ProtoError> {
    if flags & FLAG_GROUP == 0 {
        return Ok(None);
    }
    Ok(Some(GroupRef {
        group_id: r.array()?,
        epoch: r.u32()?,
    }))
}

/// Payload of an `AttachmentChunk` inner message: the attachment it belongs
/// to, its index, and the chunk sealed with the attachment key (+16-byte tag).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkPayload {
    pub attachment_id: [u8; 16],
    pub index: u16,
    pub ciphertext: Vec<u8>,
}

/// AEAD tag added to each sealed chunk.
pub const CHUNK_TAG_BYTES: usize = 16;

impl ChunkPayload {
    pub fn encode(&self) -> Result<Vec<u8>, ProtoError> {
        check_sealed_chunk(self.index, self.ciphertext.len())?;
        let mut out = Vec::with_capacity(20 + self.ciphertext.len());
        out.extend_from_slice(&self.attachment_id);
        out.extend_from_slice(&self.index.to_be_bytes());
        let len = u16::try_from(self.ciphertext.len()).map_err(|_| ProtoError::BadLength)?;
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(&self.ciphertext);
        Ok(out)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ProtoError> {
        let mut r = Reader::new(bytes);
        let attachment_id = r.array()?;
        let index = r.u16()?;
        let len = usize::from(r.u16()?);
        check_sealed_chunk(index, len)?;
        let ciphertext = r.take(len)?.to_vec();
        r.finish()?;
        Ok(Self {
            attachment_id,
            index,
            ciphertext,
        })
    }
}

fn check_sealed_chunk(index: u16, len: usize) -> Result<(), ProtoError> {
    if len <= CHUNK_TAG_BYTES || len > CHUNK_BYTES + CHUNK_TAG_BYTES {
        return Err(ProtoError::InvalidField("chunk size"));
    }
    if u32::from(index) >= chunk_count_for(MAX_FILE_BYTES) {
        return Err(ProtoError::InvalidField("chunk index"));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupMember {
    pub fingerprint: [u8; 32],
    pub name: String,
}

fn put_name(out: &mut Vec<u8>, name: &str) -> Result<(), ProtoError> {
    if name.len() > MAX_NAME_LEN {
        return Err(ProtoError::InvalidField("name"));
    }
    out.push(u8::try_from(name.len()).map_err(|_| ProtoError::BadLength)?);
    out.extend_from_slice(name.as_bytes());
    Ok(())
}

fn get_name(r: &mut Reader<'_>) -> Result<String, ProtoError> {
    let len = usize::from(r.u8()?);
    if len > MAX_NAME_LEN {
        return Err(ProtoError::InvalidField("name"));
    }
    String::from_utf8(r.take(len)?.to_vec()).map_err(|_| ProtoError::InvalidField("utf8"))
}

fn check_members(members: &[GroupMember]) -> Result<(), ProtoError> {
    if members.is_empty() || members.len() > MAX_GROUP_MEMBERS {
        return Err(ProtoError::TooManyItems);
    }
    for (i, m) in members.iter().enumerate() {
        if members
            .iter()
            .skip(i + 1)
            .any(|o| o.fingerprint == m.fingerprint)
        {
            return Err(ProtoError::InvalidField("duplicate member"));
        }
    }
    Ok(())
}

fn check_attachment(kind: u8, total_len: u32, name: &str, caption: &str) -> Result<(), ProtoError> {
    let cap = match kind {
        ATTACHMENT_IMAGE => MAX_IMAGE_BYTES,
        ATTACHMENT_FILE => MAX_FILE_BYTES,
        _ => return Err(ProtoError::InvalidField("attachment kind")),
    };
    if total_len == 0 || total_len > cap {
        return Err(ProtoError::InvalidField("attachment size"));
    }
    if name.len() > MAX_ATTACHMENT_NAME || caption.len() > MAX_CAPTION_BYTES {
        return Err(ProtoError::InvalidField("attachment text"));
    }
    Ok(())
}

fn check_timer(t: u32) -> Result<u32, ProtoError> {
    if t == 0 || (MIN_TIMER_SECS..=MAX_TIMER_SECS).contains(&t) {
        Ok(t)
    } else {
        Err(ProtoError::InvalidField("timer"))
    }
}

fn put_text(out: &mut Vec<u8>, text: &str) -> Result<(), ProtoError> {
    let len = u16::try_from(text.len()).map_err(|_| ProtoError::BadLength)?;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(text.as_bytes());
    Ok(())
}

fn get_text(r: &mut Reader<'_>) -> Result<String, ProtoError> {
    let len = usize::from(r.u16()?);
    String::from_utf8(r.take(len)?.to_vec()).map_err(|_| ProtoError::InvalidField("utf8"))
}

impl Content {
    pub fn encode(&self) -> Result<Vec<u8>, ProtoError> {
        let mut out = vec![CONTENT_VERSION];
        match self {
            Self::Text {
                reply_to,
                forwarded,
                timer_secs,
                text,
            } => {
                out.push(CT_TEXT);
                let flags = if reply_to.is_some() { FLAG_REPLY } else { 0 }
                    | if *forwarded { FLAG_FORWARDED } else { 0 };
                out.push(flags);
                if let Some(id) = reply_to {
                    out.extend_from_slice(id);
                }
                out.extend_from_slice(&check_timer(*timer_secs)?.to_be_bytes());
                put_text(&mut out, text)?;
            }
            Self::Reaction { target, reaction } => {
                if usize::from(*reaction) > REACTIONS.len() {
                    return Err(ProtoError::InvalidField("reaction"));
                }
                out.push(CT_REACTION);
                out.extend_from_slice(target);
                out.push(*reaction);
            }
            Self::Edit { target, seq, text } => {
                if !(1..=MAX_EDIT_SEQ).contains(seq) || text.is_empty() {
                    return Err(ProtoError::InvalidField("edit"));
                }
                out.push(CT_EDIT);
                out.extend_from_slice(target);
                out.push(*seq);
                put_text(&mut out, text)?;
            }
            Self::Delete { target } => {
                out.push(CT_DELETE);
                out.extend_from_slice(target);
            }
            Self::Timer { timer_secs } => {
                out.push(CT_TIMER);
                out.extend_from_slice(&check_timer(*timer_secs)?.to_be_bytes());
            }
            Self::GroupState {
                group_id,
                epoch,
                name,
                members,
            } => {
                check_members(members)?;
                out.push(CT_GROUP_STATE);
                out.extend_from_slice(group_id);
                out.extend_from_slice(&epoch.to_be_bytes());
                put_name(&mut out, name)?;
                out.push(u8::try_from(members.len()).map_err(|_| ProtoError::TooManyItems)?);
                for m in members {
                    out.extend_from_slice(&m.fingerprint);
                    put_name(&mut out, &m.name)?;
                }
            }
            Self::GroupText {
                group_id,
                group_msg_id,
                epoch,
                reply_to,
                text,
            } => {
                out.push(CT_GROUP_TEXT);
                out.extend_from_slice(group_id);
                out.extend_from_slice(group_msg_id);
                out.extend_from_slice(&epoch.to_be_bytes());
                out.push(if reply_to.is_some() { FLAG_REPLY } else { 0 });
                if let Some(id) = reply_to {
                    out.extend_from_slice(id);
                }
                put_text(&mut out, text)?;
            }
            Self::GroupLeave { group_id } => {
                out.push(CT_GROUP_LEAVE);
                out.extend_from_slice(group_id);
            }
            Self::Attachment {
                attachment_id,
                kind,
                total_len,
                sha256,
                key,
                reply_to,
                name,
                caption,
            } => {
                check_attachment(*kind, *total_len, name, caption)?;
                out.push(CT_ATTACHMENT);
                out.extend_from_slice(attachment_id);
                out.push(*kind);
                out.extend_from_slice(&total_len.to_be_bytes());
                out.extend_from_slice(sha256);
                out.extend_from_slice(key);
                out.push(if reply_to.is_some() { FLAG_REPLY } else { 0 });
                if let Some(id) = reply_to {
                    out.extend_from_slice(id);
                }
                out.push(u8::try_from(name.len()).map_err(|_| ProtoError::BadLength)?);
                out.extend_from_slice(name.as_bytes());
                put_text(&mut out, caption)?;
            }
            Self::Location {
                lat_e7,
                lon_e7,
                accuracy_m,
                captured_ms,
            } => {
                check_location(*lat_e7, *lon_e7)?;
                out.push(CT_LOCATION);
                out.extend_from_slice(&lat_e7.to_be_bytes());
                out.extend_from_slice(&lon_e7.to_be_bytes());
                out.extend_from_slice(&accuracy_m.to_be_bytes());
                out.extend_from_slice(&captured_ms.to_be_bytes());
            }
            Self::Poll {
                group,
                poll_id,
                question,
                options,
                multi,
            } => {
                check_poll(question, options)?;
                out.push(CT_POLL);
                let mut flags = 0;
                if group.is_some() {
                    flags |= FLAG_GROUP;
                }
                if *multi {
                    flags |= FLAG_MULTI;
                }
                out.push(flags);
                put_group(&mut out, group.as_ref());
                out.extend_from_slice(poll_id);
                put_text(&mut out, question)?;
                out.push(u8::try_from(options.len()).map_err(|_| ProtoError::BadLength)?);
                for o in options {
                    out.push(u8::try_from(o.len()).map_err(|_| ProtoError::BadLength)?);
                    out.extend_from_slice(o.as_bytes());
                }
            }
            Self::IntroAsk {
                intro_id,
                other_fp,
                other_name,
            } => {
                if other_name.len() > crate::MAX_NAME_LEN {
                    return Err(ProtoError::InvalidField("intro name"));
                }
                out.push(CT_INTRO_ASK);
                out.extend_from_slice(intro_id);
                out.extend_from_slice(other_fp);
                out.push(u8::try_from(other_name.len()).map_err(|_| ProtoError::BadLength)?);
                out.extend_from_slice(other_name.as_bytes());
            }
            Self::IntroCard { intro_id, card } | Self::IntroDeliver { intro_id, card } => {
                if card.is_empty() || card.len() > MAX_INTRO_CARD_BYTES {
                    return Err(ProtoError::InvalidField("intro card"));
                }
                out.push(if matches!(self, Self::IntroCard { .. }) {
                    CT_INTRO_CARD
                } else {
                    CT_INTRO_DELIVER
                });
                out.extend_from_slice(intro_id);
                out.extend_from_slice(
                    &u16::try_from(card.len())
                        .map_err(|_| ProtoError::BadLength)?
                        .to_be_bytes(),
                );
                out.extend_from_slice(card);
            }
            Self::PollVote {
                group,
                poll_id,
                choices,
            } => {
                out.push(CT_POLL_VOTE);
                out.push(if group.is_some() { FLAG_GROUP } else { 0 });
                put_group(&mut out, group.as_ref());
                out.extend_from_slice(poll_id);
                out.extend_from_slice(&choices.to_be_bytes());
            }
        }
        Ok(out)
    }

    /// Strict decode: unknown content types are `UnknownType` so the caller can
    /// still acknowledge delivery while ignoring the content.
    pub fn decode(bytes: &[u8]) -> Result<Self, ProtoError> {
        let mut r = Reader::new(bytes);
        let version = r.u8()?;
        if version != CONTENT_VERSION {
            return Err(ProtoError::UnsupportedVersion(version));
        }
        let content = match r.u8()? {
            CT_TEXT => {
                let flags = r.u8()?;
                if flags & !(FLAG_REPLY | FLAG_FORWARDED) != 0 {
                    return Err(ProtoError::ReservedBitsSet);
                }
                let reply_to = if flags & FLAG_REPLY != 0 {
                    Some(r.array()?)
                } else {
                    None
                };
                Self::Text {
                    reply_to,
                    forwarded: flags & FLAG_FORWARDED != 0,
                    timer_secs: check_timer(r.u32()?)?,
                    text: get_text(&mut r)?,
                }
            }
            CT_REACTION => {
                let target = r.array()?;
                let reaction = r.u8()?;
                if usize::from(reaction) > REACTIONS.len() {
                    return Err(ProtoError::InvalidField("reaction"));
                }
                Self::Reaction { target, reaction }
            }
            CT_EDIT => {
                let target = r.array()?;
                let seq = r.u8()?;
                let text = get_text(&mut r)?;
                if !(1..=MAX_EDIT_SEQ).contains(&seq) || text.is_empty() {
                    return Err(ProtoError::InvalidField("edit"));
                }
                Self::Edit { target, seq, text }
            }
            CT_DELETE => Self::Delete { target: r.array()? },
            CT_TIMER => Self::Timer {
                timer_secs: check_timer(r.u32()?)?,
            },
            CT_GROUP_STATE => {
                let group_id = r.array()?;
                let epoch = r.u32()?;
                let name = get_name(&mut r)?;
                let n = usize::from(r.u8()?);
                if n == 0 || n > MAX_GROUP_MEMBERS {
                    return Err(ProtoError::TooManyItems);
                }
                let mut members = Vec::with_capacity(n);
                for _ in 0..n {
                    members.push(GroupMember {
                        fingerprint: r.array()?,
                        name: get_name(&mut r)?,
                    });
                }
                check_members(&members)?;
                Self::GroupState {
                    group_id,
                    epoch,
                    name,
                    members,
                }
            }
            CT_GROUP_TEXT => {
                let group_id = r.array()?;
                let group_msg_id = r.array()?;
                let epoch = r.u32()?;
                let flags = r.u8()?;
                if flags & !FLAG_REPLY != 0 {
                    return Err(ProtoError::ReservedBitsSet);
                }
                let reply_to = if flags & FLAG_REPLY != 0 {
                    Some(r.array()?)
                } else {
                    None
                };
                Self::GroupText {
                    group_id,
                    group_msg_id,
                    epoch,
                    reply_to,
                    text: get_text(&mut r)?,
                }
            }
            CT_GROUP_LEAVE => Self::GroupLeave {
                group_id: r.array()?,
            },
            CT_ATTACHMENT => {
                let attachment_id = r.array()?;
                let kind = r.u8()?;
                let total_len = r.u32()?;
                let sha256 = r.array()?;
                let key = r.array()?;
                let flags = r.u8()?;
                if flags & !FLAG_REPLY != 0 {
                    return Err(ProtoError::ReservedBitsSet);
                }
                let reply_to = if flags & FLAG_REPLY != 0 {
                    Some(r.array()?)
                } else {
                    None
                };
                let name_len = usize::from(r.u8()?);
                if name_len > MAX_ATTACHMENT_NAME {
                    return Err(ProtoError::InvalidField("attachment text"));
                }
                let name = String::from_utf8(r.take(name_len)?.to_vec())
                    .map_err(|_| ProtoError::InvalidField("utf8"))?;
                let caption_len = usize::from(r.u16()?);
                if caption_len > MAX_CAPTION_BYTES {
                    return Err(ProtoError::InvalidField("attachment text"));
                }
                let caption = String::from_utf8(r.take(caption_len)?.to_vec())
                    .map_err(|_| ProtoError::InvalidField("utf8"))?;
                check_attachment(kind, total_len, &name, &caption)?;
                Self::Attachment {
                    attachment_id,
                    kind,
                    total_len,
                    sha256,
                    key,
                    reply_to,
                    name,
                    caption,
                }
            }
            CT_LOCATION => {
                let lat_e7 = i32::from_be_bytes(r.array()?);
                let lon_e7 = i32::from_be_bytes(r.array()?);
                let accuracy_m = r.u16()?;
                let captured_ms = r.u64()?;
                check_location(lat_e7, lon_e7)?;
                Self::Location {
                    lat_e7,
                    lon_e7,
                    accuracy_m,
                    captured_ms,
                }
            }
            CT_POLL => {
                let flags = r.u8()?;
                if flags & !(FLAG_GROUP | FLAG_MULTI) != 0 {
                    return Err(ProtoError::ReservedBitsSet);
                }
                let group = get_group(&mut r, flags)?;
                let poll_id = r.array()?;
                let question = get_text(&mut r)?;
                let n = usize::from(r.u8()?);
                if n > MAX_POLL_OPTIONS {
                    return Err(ProtoError::InvalidField("poll"));
                }
                let mut options = Vec::with_capacity(n);
                for _ in 0..n {
                    let len = usize::from(r.u8()?);
                    options.push(
                        String::from_utf8(r.take(len)?.to_vec())
                            .map_err(|_| ProtoError::InvalidField("utf8"))?,
                    );
                }
                let multi = flags & FLAG_MULTI != 0;
                check_poll(&question, &options)?;
                Self::Poll {
                    group,
                    poll_id,
                    question,
                    options,
                    multi,
                }
            }
            CT_POLL_VOTE => {
                let flags = r.u8()?;
                if flags & !FLAG_GROUP != 0 {
                    return Err(ProtoError::ReservedBitsSet);
                }
                let group = get_group(&mut r, flags)?;
                Self::PollVote {
                    group,
                    poll_id: r.array()?,
                    choices: r.u16()?,
                }
            }
            CT_INTRO_ASK => {
                let intro_id = r.array()?;
                let other_fp = r.array()?;
                let n = usize::from(r.u8()?);
                if n > crate::MAX_NAME_LEN {
                    return Err(ProtoError::InvalidField("intro name"));
                }
                let other_name = String::from_utf8(r.take(n)?.to_vec())
                    .map_err(|_| ProtoError::InvalidField("utf8"))?;
                Self::IntroAsk {
                    intro_id,
                    other_fp,
                    other_name,
                }
            }
            kind @ (CT_INTRO_CARD | CT_INTRO_DELIVER) => {
                let intro_id = r.array()?;
                let n = usize::from(r.u16()?);
                if n == 0 || n > MAX_INTRO_CARD_BYTES {
                    return Err(ProtoError::InvalidField("intro card"));
                }
                let card = r.take(n)?.to_vec();
                if kind == CT_INTRO_CARD {
                    Self::IntroCard { intro_id, card }
                } else {
                    Self::IntroDeliver { intro_id, card }
                }
            }
            other => return Err(ProtoError::UnknownType(other)),
        };
        r.finish()?;
        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn samples() -> Vec<Content> {
        vec![
            Content::Text {
                reply_to: Some([3; 16]),
                forwarded: true,
                timer_secs: 3600,
                text: "héllo".into(),
            },
            Content::Text {
                reply_to: None,
                forwarded: false,
                timer_secs: 0,
                text: String::new(),
            },
            Content::Reaction {
                target: [4; 16],
                reaction: 8,
            },
            Content::Reaction {
                target: [4; 16],
                reaction: 0,
            },
            Content::Edit {
                target: [5; 16],
                seq: 10,
                text: "fixed".into(),
            },
            Content::Delete { target: [6; 16] },
            Content::Timer { timer_secs: 60 },
            Content::GroupState {
                group_id: [7; 16],
                epoch: 3,
                name: "Hikers".into(),
                members: (0..MAX_GROUP_MEMBERS as u8)
                    .map(|i| GroupMember {
                        fingerprint: [i; 32],
                        name: "x".repeat(MAX_NAME_LEN),
                    })
                    .collect(),
            },
            Content::GroupText {
                group_id: [7; 16],
                group_msg_id: [8; 16],
                epoch: 3,
                reply_to: Some([9; 16]),
                text: "at the trailhead".into(),
            },
            Content::GroupLeave { group_id: [7; 16] },
            Content::Attachment {
                attachment_id: [4; 16],
                kind: ATTACHMENT_IMAGE,
                total_len: 3000,
                sha256: [5; 32],
                key: [8; 32],
                reply_to: Some([6; 16]),
                name: "gate.jpg".into(),
                caption: "the north gate".into(),
            },
            Content::Attachment {
                attachment_id: [4; 16],
                kind: ATTACHMENT_FILE,
                total_len: MAX_FILE_BYTES,
                sha256: [5; 32],
                key: [8; 32],
                reply_to: None,
                name: String::new(),
                caption: String::new(),
            },
        ]
    }

    #[test]
    fn attachment_limits() {
        let att = |kind, total_len, name: &str, caption: &str| Content::Attachment {
            attachment_id: [1; 16],
            kind,
            total_len,
            sha256: [0; 32],
            key: [0; 32],
            reply_to: None,
            name: name.into(),
            caption: caption.into(),
        };
        for bad in [
            att(ATTACHMENT_IMAGE, MAX_IMAGE_BYTES + 1, "", ""),
            att(ATTACHMENT_FILE, MAX_FILE_BYTES + 1, "", ""),
            att(ATTACHMENT_FILE, 0, "", ""),
            att(3, 10, "", ""),
            att(
                ATTACHMENT_FILE,
                10,
                &"n".repeat(MAX_ATTACHMENT_NAME + 1),
                "",
            ),
            att(ATTACHMENT_FILE, 10, "", &"c".repeat(MAX_CAPTION_BYTES + 1)),
        ] {
            assert!(bad.encode().is_err(), "{bad:?}");
        }
        let chunk = |index, len| ChunkPayload {
            attachment_id: [1; 16],
            index,
            ciphertext: vec![0; len],
        };
        assert!(chunk(0, CHUNK_TAG_BYTES).encode().is_err(), "tag only");
        assert!(
            chunk(0, CHUNK_BYTES + CHUNK_TAG_BYTES + 1)
                .encode()
                .is_err()
        );
        let last = u16::try_from(chunk_count_for(MAX_FILE_BYTES)).unwrap();
        assert!(chunk(last, 20).encode().is_err());
        let ok = chunk(last - 1, CHUNK_BYTES + CHUNK_TAG_BYTES);
        assert_eq!(ChunkPayload::decode(&ok.encode().unwrap()).unwrap(), ok);
        let mut trailing = ok.encode().unwrap();
        trailing.push(0);
        assert_eq!(
            ChunkPayload::decode(&trailing),
            Err(ProtoError::TrailingBytes)
        );
        // A declared size above the cap is rejected on decode too.
        let mut bytes = att(ATTACHMENT_IMAGE, 100, "", "").encode().unwrap();
        bytes[2 + 16 + 1..2 + 16 + 5].copy_from_slice(&(MAX_IMAGE_BYTES + 1).to_be_bytes());
        assert_eq!(
            Content::decode(&bytes),
            Err(ProtoError::InvalidField("attachment size"))
        );
        // A chunk length field larger than the cap is rejected before allocation.
        let mut big = chunk(0, 20).encode().unwrap();
        big[16 + 2..16 + 4].copy_from_slice(&u16::MAX.to_be_bytes());
        assert_eq!(
            ChunkPayload::decode(&big),
            Err(ProtoError::InvalidField("chunk size"))
        );
        assert_eq!(chunk_count_for(MAX_FILE_BYTES), 47);
        assert_eq!(chunk_count_for(1), 1);
    }

    #[test]
    fn round_trips() {
        for c in samples() {
            assert_eq!(Content::decode(&c.encode().unwrap()).unwrap(), c);
        }
    }

    #[test]
    fn rejects_out_of_range_fields() {
        let bad = [
            Content::Reaction {
                target: [0; 16],
                reaction: 9,
            },
            Content::Edit {
                target: [0; 16],
                seq: 0,
                text: "x".into(),
            },
            Content::Edit {
                target: [0; 16],
                seq: 11,
                text: "x".into(),
            },
            Content::Edit {
                target: [0; 16],
                seq: 1,
                text: String::new(),
            },
            Content::Timer { timer_secs: 59 },
            Content::Timer {
                timer_secs: MAX_TIMER_SECS + 1,
            },
        ];
        for c in bad {
            assert!(c.encode().is_err(), "{c:?}");
        }
        let mut reaction = Content::Reaction {
            target: [0; 16],
            reaction: 1,
        }
        .encode()
        .unwrap();
        *reaction.last_mut().unwrap() = 9;
        assert_eq!(
            Content::decode(&reaction),
            Err(ProtoError::InvalidField("reaction"))
        );
    }

    #[test]
    fn rejects_malformed_bytes() {
        let text = samples().remove(0).encode().unwrap();
        let mut trailing = text.clone();
        trailing.push(0);
        assert_eq!(Content::decode(&trailing), Err(ProtoError::TrailingBytes));
        assert_eq!(
            Content::decode(&text[..text.len() - 1]),
            Err(ProtoError::Truncated)
        );
        let mut reserved = text.clone();
        reserved[2] |= 0x80;
        assert_eq!(Content::decode(&reserved), Err(ProtoError::ReservedBitsSet));
        let mut bad_utf8 = Content::Text {
            reply_to: None,
            forwarded: false,
            timer_secs: 0,
            text: "ab".into(),
        }
        .encode()
        .unwrap();
        let n = bad_utf8.len();
        bad_utf8[n - 1] = 0xff;
        assert_eq!(
            Content::decode(&bad_utf8),
            Err(ProtoError::InvalidField("utf8"))
        );
        assert_eq!(
            Content::decode(&[1, 0x7f]),
            Err(ProtoError::UnknownType(0x7f))
        );
        assert_eq!(
            Content::decode(&[2, CT_DELETE]),
            Err(ProtoError::UnsupportedVersion(2))
        );
    }

    #[test]
    fn introductions_round_trip_and_enforce_limits() {
        let ask = Content::IntroAsk {
            intro_id: [1; 16],
            other_fp: [2; 32],
            other_name: "Ravi".into(),
        };
        assert_eq!(Content::decode(&ask.encode().unwrap()).unwrap(), ask);
        let card = Content::IntroCard {
            intro_id: [1; 16],
            card: vec![9; 300],
        };
        assert_eq!(Content::decode(&card.encode().unwrap()).unwrap(), card);
        let deliver = Content::IntroDeliver {
            intro_id: [1; 16],
            card: vec![8; 300],
        };
        assert_eq!(
            Content::decode(&deliver.encode().unwrap()).unwrap(),
            deliver
        );
        assert!(
            Content::IntroCard {
                intro_id: [1; 16],
                card: vec![]
            }
            .encode()
            .is_err()
        );
        assert!(
            Content::IntroCard {
                intro_id: [1; 16],
                card: vec![0; MAX_INTRO_CARD_BYTES + 1]
            }
            .encode()
            .is_err()
        );
        let long = Content::IntroAsk {
            intro_id: [1; 16],
            other_fp: [2; 32],
            other_name: "x".repeat(200),
        };
        assert!(long.encode().is_err());
    }

    #[test]
    fn location_and_polls_round_trip_and_enforce_limits() {
        let loc = Content::Location {
            lat_e7: 174_123_456,
            lon_e7: 785_123_456,
            accuracy_m: 12,
            captured_ms: 1_790_000_000_000,
        };
        assert_eq!(Content::decode(&loc.encode().unwrap()).unwrap(), loc);
        let bad = Content::Location {
            lat_e7: 900_000_001,
            lon_e7: 0,
            accuracy_m: 0,
            captured_ms: 0,
        };
        assert!(bad.encode().is_err());

        let group = Some(GroupRef {
            group_id: [3; 16],
            epoch: 2,
        });
        let poll = Content::Poll {
            group,
            poll_id: [7; 16],
            question: "Where do we meet?".into(),
            options: vec!["North gate".into(), "Library".into()],
            multi: true,
        };
        assert_eq!(Content::decode(&poll.encode().unwrap()).unwrap(), poll);
        let vote = Content::PollVote {
            group: None,
            poll_id: [7; 16],
            choices: 0b10,
        };
        assert_eq!(Content::decode(&vote.encode().unwrap()).unwrap(), vote);

        let poll_with = |q: &str, opts: Vec<&str>| Content::Poll {
            group: None,
            poll_id: [1; 16],
            question: q.into(),
            options: opts.into_iter().map(String::from).collect(),
            multi: false,
        };
        assert!(
            poll_with("Q", vec!["only one"]).encode().is_err(),
            "needs two options"
        );
        assert!(
            poll_with("Q", vec!["a"; MAX_POLL_OPTIONS + 1])
                .encode()
                .is_err()
        );
        assert!(
            poll_with(" ", vec!["a", "b"]).encode().is_err(),
            "empty question"
        );
        let long = "x".repeat(MAX_POLL_OPTION_BYTES + 1);
        assert!(poll_with("Q", vec!["a", &long]).encode().is_err());
    }

    #[test]
    fn group_state_limits() {
        let m = |b: u8| GroupMember {
            fingerprint: [b; 32],
            name: "m".into(),
        };
        let state = |members: Vec<GroupMember>, name: &str| Content::GroupState {
            group_id: [1; 16],
            epoch: 1,
            name: name.into(),
            members,
        };
        assert_eq!(state(vec![], "g").encode(), Err(ProtoError::TooManyItems));
        assert_eq!(
            state((0..9).map(m).collect(), "g").encode(),
            Err(ProtoError::TooManyItems)
        );
        assert_eq!(
            state(vec![m(1), m(1)], "g").encode(),
            Err(ProtoError::InvalidField("duplicate member"))
        );
        assert_eq!(
            state(vec![m(1)], &"n".repeat(MAX_NAME_LEN + 1)).encode(),
            Err(ProtoError::InvalidField("name"))
        );
        let mut nine = state((0..8).map(m).collect(), "g").encode().unwrap();
        nine[2 + 16 + 4 + 2] = 9;
        assert_eq!(Content::decode(&nine), Err(ProtoError::TooManyItems));
        let mut dup = state(vec![m(1), m(2)], "g").encode().unwrap();
        let second = 2 + 16 + 4 + 2 + 1 + 32 + 2;
        dup[second..second + 32].copy_from_slice(&[1; 32]);
        assert_eq!(
            Content::decode(&dup),
            Err(ProtoError::InvalidField("duplicate member"))
        );
    }
}
