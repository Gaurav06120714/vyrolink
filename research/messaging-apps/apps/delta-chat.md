# Delta Chat (chatmail) — research report

**Accessed:** 2026-10-15. **Source code inspected:** `chatmail/core` (Rust core) @ `64e5875` (2026-10-14, MPL-2.0; studied only).

**Overall confidence:** High for the code cited.

## 1. Why it matters

Delta Chat's core is **a Rust library with SQLite storage and FFI bindings** driving native/UI front-ends, the same shape as VYRO LINK. Its transport (email/IMAP) is asynchronous and store-and-forward, so messages arrive **late and out of order**. Its edit, delete and ephemeral rules are tested answers to exactly our ordering problem.

## 2. Capabilities (VERIFIED in source unless noted)

| Capability | Behaviour | Where |
|---|---|---|
| Reactions | RFC 9078 transport with **XEP-0444 update semantics**: the latest reaction from a user **replaces** all earlier ones; an empty reaction removes it | `src/reaction.rs` module docs |
| Edit | Sender-only (`from_id` must match); an edit of an encrypted message must itself be encrypted; an edit for an unknown message is **dropped** with a warning; text-only, non-empty; the original quote is kept for legacy clients | `chat.rs::send_edit_request`, `receive_imf.rs::handle_edit_delete` |
| Delete for everyone | Sender-only; **encrypted only**. If the target is unknown, a **tombstone** is inserted so the message is ignored if it arrives later | `receive_imf.rs::handle_edit_delete` |
| Local delete | Moves the message to a "trash" chat as a **tombstone** (contents removed, ID kept) to prevent re-download; tombstones pruned later | `ephemeral.rs` docs |
| Ephemeral messages | Per-chat timer, synchronised by applying the timer value from **every received message**; the timer **starts when the message is seen**; a separate device-only "delete after" setting | `ephemeral.rs` docs |
| Pinned messages | Yes | `src/pinned_messages.rs` exists |
| Security analysis | ETH Zurich, USENIX Security 2024: 5 attacks from cross-protocol interactions (SecureJoin × Autocrypt) and rPGP bugs; all main ones fixed | REPORTED (eprint 2024/918) |
| Audits | "Six independent security audits and analyses" | REPORTED (Pass the SALT 2025 talk abstract) |

## 3. Lessons

**Adopt:**
- **tombstones for out-of-order delete** (critical for a delay-tolerant network);
- sender-only edit/delete, checked against the authenticated sender;
- edit/delete must be at least as protected as the original;
- latest-reaction-wins per user, with an empty value to remove;
- the ephemeral timer starts on *seen* for incoming messages.

**Differ:** Delta Chat **drops** edits that arrive before their target. Store-and-forward makes that common, so VYRO LINK should **hold a pending edit** (bounded) and apply it when the target arrives (see 08 acceptance criteria).

**Avoid:** cross-protocol interactions. ETH's root cause was "unclear specifications and cross-protocol interactions", so every new inner kind must be specified in Doc 21 and covered by test vectors.

## 4. Sources (accessed 2026-10-15)
- Source: `references/chatmail-core` @ 64e5875, `src/reaction.rs`, `src/ephemeral.rs`, `src/chat.rs`, `src/receive_imf.rs`. High confidence.
- Yuanming Song et al. (ETH Zurich), "Cryptographic Analysis of Delta Chat", USENIX Security 2024: https://eprint.iacr.org/2024/918 (high).
- Pass the SALT 2025 talk: https://passthesalt.ubicast.tv/permalink/v126afbe812cai76emve/ (medium).
