# 07 — Feature Implementation Roadmap

**Created:** 2026-10-15. The **live status** is in `09-feature-implementation-tracker.md`; this file holds the order and the dependencies.

## Stage A — device-independent core features (now)

1. **Wire:** inner kind `0x11 CONTENT` with ctypes TEXT, REACTION, EDIT, DELETE, TIMER.
   - Spec in Doc 21 §4.1; vectors; fuzz target coverage (`decode_all`).
2. **Storage:** Class C migration 2.
   - Messages gain `sent_at_ms`, `reply_to`, `forwarded`, `deleted`, `timer_secs`, `read_at`, `expire_at`, `pinned`, `starred`.
   - New tables: `reactions`, `edits`, `tombstones`, `drafts`, `conversation_settings`, and an FTS5 table `messages_fts`.
   - Contacts gain `blocked`.
   - O(1) message lookups replace the full-conversation scans.
3. **Core API** (`vl-core`):
   - `send_text_ex` (reply/forward), `react`, `delete_for_me`, `delete_for_everyone`, `edit_message`;
   - `set_disappearing_timer`, `mark_read`, `search`;
   - `save_draft`, `draft`, `block`, `unblock`;
   - pins and bookmarks.
4. **FFI** (`vl-ffi`): expose the API; extend the Swift interop test.
5. **Tests:** every *(auto)* criterion in 08.
6. **Simulator:** measure SUMMARY airtime share (A-3); compare the receipt-return options (F-18).

## Stage B — needs the Android SDK (blocked on licence acceptance)

- Kotlin binding compile + run; Android cross-build of vl-ffi (SQLCipher vendored OpenSSL).

## Stage C — needs devices (blocked on hardware)

- Phase 5 BLE probes → Experiments A, B, C (Doc 14).
- AC-W.5 and every *(device)* criterion.

## Stage D — UI (only after the underlying feature is TESTED, and per the Doc 14 gates)

- The Flutter UI shows features that are **TESTED**; nothing mocked is presented as real.
