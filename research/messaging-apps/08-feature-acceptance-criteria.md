# 08 — Feature Acceptance Criteria

A feature reaches **TESTED** only when every criterion marked *(auto)* has a passing automated test that is named in 09.

**DEVICE_VALIDATED** additionally needs the *(device)* criteria, verified on physical phones. Device criteria are BLOCKED until hardware is available.

**General criteria for every feature:**
- **G-1:** no plaintext, keys, tags or IDs in logs *(auto: plaintext-canary DB test; review)*.
- **G-2:** every new decoder is length- and count-bounded and fuzzed *(auto)*.
- **G-3:** golden vectors for new wire bytes *(auto)*.
- **G-4:** `cargo fmt`, Clippy (0 warnings), deny, audit, tests and Swift interop all green.

## F-03 Replies

- AC-03.1 *(auto)*: a reply carries only the 16-byte target ID; the quoted text is **not** transmitted.
- AC-03.2 *(auto)*: the recipient stores `reply_to`; the target may be unknown at arrival (stored anyway).
- AC-03.3 *(auto)*: a reply to a deleted target still delivers; the UI layer gets `reply_to` plus the target's deleted flag.

## F-04 Reactions

- AC-04.1 *(auto)*: only reaction indices 0–8 are accepted (0 = remove; 1–8 map to 👍 👎 😀 😂 😢 ❤️ 🚀 ✅); others → decode error.
- AC-04.2 *(auto)*: per (target, reactor), the event with the greatest `(sent_at_ms, message_id)` wins **regardless of arrival order** (both orders tested).
- AC-04.3 *(auto)*: a reaction arriving before its target is shown once the target arrives.
- AC-04.4 *(auto)*: replaying the same reaction event is idempotent.
- AC-04.5 *(auto)*: a reaction to a message from another conversation is ignored.

## F-05 Delete

- AC-05.1 *(auto)*: delete-for-me removes the body, reactions, edits and search entry locally only.
- AC-05.2 *(auto)*: delete-for-everyone is accepted only from the target's original sender.
  - A contact cannot delete my messages.
  - A contact's delete for another contact's message is ignored.
- AC-05.3 *(auto)*: a delete arriving **before** its target creates a tombstone; the target, when it arrives, is stored with an empty body and `deleted = true`.
- AC-05.4 *(auto)*: a delete outside the 72 h sender-time window is ignored.
- AC-05.5 *(auto)*: idempotent on replay; the search index no longer returns the message.
- AC-05.6 *(UI)*: wording says "deletion requested" and shows a placeholder, not silent disappearance.

## F-06 Search

- AC-06.1 *(auto)*: finds messages by word and prefix (case-insensitive) within a conversation or across all.
- AC-06.2 *(auto)*: FTS operators in user input (`"`, `*`, `NEAR`, `-`, `:`, parentheses) cannot cause a query error or injection; they are treated as literal terms.
- AC-06.3 *(auto)*: deleted, disappeared and edited-away text is not found; edited text is found.
- AC-06.4 *(auto)*: the index is inside the encrypted DB (plaintext canary not on disk).

## F-07 Drafts

- AC-07.1 *(auto)*: save, load and clear one draft per contact; survives reopen; an empty draft deletes the row.
- AC-07.2 *(auto)*: size is bounded by `MAX_TEXT_BYTES`.

## F-08 Blocking

- AC-08.1 *(auto)*: blocking removes the contact's inbound tags from the relay's own-tag set, so their envelopes are not requested for delivery.
- AC-08.2 *(auto)*: an envelope from a blocked contact that still arrives is dropped: no message, no receipt, no vaccine.
- AC-08.3 *(auto)*: sending to a blocked contact fails with `ContactBlocked`.
- AC-08.4 *(auto)*: unblocking restores tags; **new** messages are delivered again. Messages sent while blocked stay undelivered: the relay engine may already have carried them as opaque copies and marked them seen. This matches mainstream semantics (WhatsApp says messages from a blocked contact are never delivered; REPORTED).
- AC-08.5 *(auto)*: other contacts' envelopes are still relayed (blocking never affects carrying for others).

## F-09 Disappearing messages

- AC-09.1 *(auto)*: the timer is 0 (off) or between 60 s and 365 d; anything else → decode error.
- AC-09.2 *(auto)*: every outgoing CONTENT TEXT carries the conversation's current timer; the recipient adopts a received timer only if its `sent_at_ms` is newer than the last timer change (last-writer-wins).
- AC-09.3 *(auto)*: outgoing messages expire at `created_at + timer`; incoming messages expire at `read_at + timer` (the timer starts on read).
- AC-09.4 *(auto)*: `maintain()` deletes expired messages with their reactions, edits and search entries.
- AC-09.5 *(UI)*: wording says "deleted from this device"; no claims about screenshots or the other device.

## F-10 Edit (Tier 2)

- AC-10.1 *(auto)*: only the original sender's edits apply; `seq` is between 1 and 10; sender time is within 24 h of the target.
- AC-10.2 *(auto)*: an edit arriving before its target is applied when the target arrives.
- AC-10.3 *(auto)*: the highest `seq` wins regardless of order; the result exposes `edited = true`.
- AC-10.4 *(auto)*: an edit of a deleted message has no effect.

## Shared wire criteria (CONTENT kind 0x11)

- AC-W.1 *(auto)*: the largest allowed text (1,700 B) with a reply and a timer fits the 2,048 bucket as an Olm **pre-key** message.
- AC-W.2 *(auto)*: an unknown `ctype` with a valid Olm payload → RECEIPT_DELIVERED is still produced, and nothing is shown except an "unsupported" counter.
- AC-W.3 *(auto)*: trailing bytes, an oversized `text_len`, non-UTF-8 text or non-zero reserved flag bits → decode error.
- AC-W.4 *(auto)*: legacy TEXT `0x10` messages still decode (backward compatibility).
- AC-W.5 *(device, BLOCKED)*: features round-trip between a physical iPhone and Android phone across at least one relay hop.

## F-16 Panic wipe

**Specification:** a panic wipe must make all local data unrecoverable as quickly as possible.
- **Mechanism:** cryptographic erasure plus file removal. Overwriting files on flash storage with WAL is not reliable, so the host deletes both database keys from the Keychain/Keystore first; the core then closes everything and deletes the files.
- **Scope:**
  - identity (Olm account, addressing key), contacts, sessions, messages, drafts, search index;
  - pairwise keys and tags;
  - **envelopes carried for other people** (their custody is lost; the same choice as bitchat's panic wipe).
- **Trigger UI** (button, gesture, duress PIN) is part of Phase 8 and not built.

- AC-16.1 *(auto)*: after `panic_wipe` the relay and conversation database files and their `-wal`, `-shm` and `-journal` side files are gone.
- AC-16.2 *(auto)*: reopening at the same paths yields a fresh node: no identity, no contacts, no carried envelopes.
- AC-16.3 *(auto)*: the FFI object refuses every later call with `Wiped`; a second wipe reports `Wiped`; the wipe works even if the lock was poisoned by an earlier panic.
- AC-16.4 *(auto)*: the wipe succeeds with an open link session and live WAL files.
- AC-16.5 *(native, BLOCKED)*: the host deletes the Keychain/Keystore items **before** calling `panic_wipe`, and the keys are verified absent afterwards. Requires the native hosts (Phase 7).
- AC-16.6 *(device, BLOCKED)*: the wipe completes within 1 s on a mid-range phone.

## F-13 Small groups (ADR-024)

- AC-13.1 *(auto)*: 2 to 8 members including the creator; members are identified by pairing-card fingerprints; duplicates are rejected.
- AC-13.2 *(auto)*: the roster changes only through the creator, with a strictly greater epoch. Non-creator and stale rosters are ignored.
- AC-13.3 *(auto)*: a group message is accepted only from a current member, and is held (bounded, with TTL) while the roster is missing or older.
- AC-13.4 *(auto)*: removed members learn of their removal and cannot post to remaining members; leaving asks the creator to remove me.
- AC-13.5 *(auto)*: one pairwise copy per paired member; the delivery count reflects only authenticated proofs.
- AC-13.6 *(device, BLOCKED)*: group delivery over physical relays.

