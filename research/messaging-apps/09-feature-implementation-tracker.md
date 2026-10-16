# 09 — Feature Implementation Tracker

**Last updated:** 2026-10-15.

**Status vocabulary:** RESEARCHING · APPROVED · IN_PROGRESS · IMPLEMENTED · TESTED · DEVICE_VALIDATED · BLOCKED · DEFERRED · REJECTED.

**Rules:**
- **TESTED** = every *(auto)* criterion in 08 has a named passing test, run locally on macOS (all in-process; only the radio is simulated).
- **DEVICE_VALIDATED** requires physical-device evidence. **No feature is DEVICE_VALIDATED**: there is no Android device and the iPhone is offline, so AC-W.5 is BLOCKED.
- **UI (2026-10-15):** the Flutter app exists. The per-feature UI status is in the "App integration" table below. "Core" rows keep their core evidence.

**Test files:**
- `S` = `core/crates/vl-store/tests/features.rs`
- `E` = `core/crates/vl-core/tests/features.rs`
- `P` = `vl-proto` (`src/content.rs`, `tests/properties.rs`, `tests/vectors.rs`)
- `W` = Swift interop (`core/bindings-tests/swift/main.swift`)

| ID | Feature | Tier | Status | Evidence (tests) | Notes |
|---|---|---|---|---|---|
| F-01 | In-person QR pairing (core) | 1 | TESTED | vl-crypto card tests incl. `low_order_keys_in_a_signed_card_are_rejected`; `three_devices.rs`; W | Camera UI not built |
| F-02 | Transparent delivery states | 1 | TESTED | `delivered_state_cannot_be_downgraded`; `three_devices.rs`; W | — |
| F-03 | Replies | 1 | TESTED | E `reply_react_edit_delete_round_trip`; P round-trip/vectors; W "reply target crosses the boundary" | Quote = target ID only |
| F-04 | Reactions (8 fixed) | 1 | TESTED | S `reactions_converge_regardless_of_arrival_order`, `reaction_removal_is_not_resurrected_by_older_event`, `reaction_from_another_conversation_is_ignored`; E round trip + removal; P range checks; W | — |
| F-05 | Delete for me / for everyone | 1 | TESTED | S `delete_for_everyone_rules` (both orders, author, window, idempotent), `delete_for_me_is_local_and_suppresses_duplicates`; **E `delete_arriving_before_its_target_via_relays`** (real spray-and-wait, 4 relays) | UI wording AC-05.6 pending (no UI) |
| F-06 | Local search (FTS5) | 1 | TESTED | S `search_is_prefix_case_insensitive_and_injection_safe`; store `plaintext_never_reaches_disk` (index encrypted); E; W | `unicode61` tokenizer (CJK weak, 05 §5). Messages stored before schema v2 are not indexed (no v1 databases exist outside tests) |
| F-07 | Drafts | 1 | TESTED | S `drafts_save_load_and_clear`; E `drafts_and_pins` (cleared on send, size bound) | — |
| F-08 | Blocking | 1 | TESTED | S `blocked_flag_round_trips`; E `blocking_drops_their_mail_but_keeps_relaying_for_others` | Mail sent while blocked stays undelivered (AC-08.4) |
| F-09 | Disappearing messages | 1 | TESTED | S `disappearing_timer_is_last_writer_wins_and_purges`; E `disappearing_messages_follow_the_newest_timer_and_vanish` | Incoming timer starts on `mark_read`; UI wording pending |
| F-10 | Edit | 2 | TESTED | S `edits_converge_and_respect_author_and_window` (both orders); E round trip + "cannot edit the contact's message"; W | — |
| F-11 | Pins / bookmarks (local) | 2 | TESTED | S `pins_and_stars_are_local_flags`; E `drafts_and_pins` | No pin expiry (Signal-style); add if wanted |
| F-12 | Forwarding (flag only) | 2 | TESTED | E round trip asserts `forwarded` and no origin; P round trip | — |
| F-13 | Small groups (pairwise fan-out, ≤ 8) | 2 | TESTED | `vl-core/tests/groups.rs` (7: create/send/delivery count, held-before-invite, non-creator and non-member rejection, stale roster replay, removal enforcement, leave, unpaired members); S `group_pending_is_bounded_and_expires`; P group vectors + limits; vl-crypto `largest_group_text_fits_as_pre_key_message`; W 3 group checks | No group reactions/edits/deletes yet; no transcript consistency (algorithms/04 §5) |
| F-14 | Small images + files | 2 | TESTED (core), built in apps 2026-10-16 | `extras.rs` attachment tests; vl-store `features.rs`; W | ADR-026. Images ≤ 16 KiB, files ≤ 64 KiB; R + W tests; NOT device-tested |
| F-15 | Offline notifications | 2 | BLOCKED | — | Native Phase 7; devices |
| F-16 | Panic wipe (core + FFI) | 2 | TESTED | E `panic_wipe_removes_databases_and_identity` (open link, WAL present, nothing left, fresh on reopen, carried mail gone); W "panic wipe deletes the database files", "wiped node refuses every call", "second wipe reports Wiped" | Key deletion in Keychain/Keystore (AC-16.5) and the trigger UI are native/UI work: BLOCKED/not started. Carried envelopes for others are destroyed |
| F-17 | Per-contact preferences | 2 | RESEARCHING | — | SimpleX model |
| F-18 | Receipt-return improvement | 2 | RESEARCHING | Simulator E1 refuted "receipt lifetime" as the cause (algorithms/03 §3) | Next: receipt budget / vaccine advertising experiments (algorithms/09 N2) |
| F-19 – F-23 | Voice notes, polls, local backup, paper QR message, introductions | 3 | DEFERRED | — | 06 |
| — | Multi-device, PQ hybrid, Megolm groups, large files, broadcast, read receipts by default | — | DEFERRED | — | 06 |
| — | Calls, cloud sync, phone discovery, typing/presence, channels | — | REJECTED | — | 06 |

## App integration (Flutter UI + native hosts, 2026-10-15)

**Statuses:**
- **IMPLEMENTED:** the screen calls the real core.
- **TESTED (widget):** a Flutter test with a fake host.
- **TESTED (sim e2e):** `integration_test/app_flow_test.dart` against the real core, passing on the iOS 27 simulator and the Android 37 emulator.
- **No row is DEVICE_VALIDATED.**

| ID | UI / native status | Evidence |
|---|---|---|
| F-01 QR pairing | IMPLEMENTED (iOS camera + Android CameraX/ZXing; QR rendered from the core). Camera scanning not exercised (the simulator has no camera) | Core `pairing_by_qr_text_and_self_scan_is_refused`; sim e2e pairs through the same core calls via the loopback peer |
| F-02 Delivery states | TESTED (widget + sim e2e): "Waiting for nearby devices" before the exchange, "Delivered" only after the authenticated receipt | `conversation: a queued message shows waiting, never Delivered`; e2e screenshots 06 and 08 |
| F-03 Replies | IMPLEMENTED (quote preview, reply bar) | — |
| F-04 Reactions | TESTED (sim e2e) | e2e "React 👍" |
| F-05 Delete | IMPLEMENTED (for me / for everyone with recall warning) | — |
| F-06 Search | IMPLEMENTED (global and per chat, highlighted) | — |
| F-07 Drafts | IMPLEMENTED (debounced save, restored on open) | — |
| F-08 Blocking | IMPLEMENTED; send failure explained | `conversation: send failure keeps the text and explains` |
| F-09 Disappearing | IMPLEMENTED (per-chat timer sheet, header indicator) | — |
| F-10 Edit | IMPLEMENTED (24 h window) | — |
| F-11 Pins/stars | IMPLEMENTED (+ lists) | — |
| F-12 Forward | IMPLEMENTED | — |
| F-13 Groups | IMPLEMENTED (create, members, leave, "Delivered to n of m") | — |
| F-14 Images | Implemented 2026-10-16 (ADR-026): attachment announcement in Olm, parts as sealed inner kind 0x12 | `core/crates/vl-core/tests/extras.rs` |
| F-15 Notifications | IMPLEMENTED (generic text only, after a Bluetooth session, app not in front) | Not device-validated |
| F-16 Panic wipe | TESTED (widget + sim e2e): Keychain keys destroyed and verified, then files; UI returns to onboarding; a key-store failure reports that nothing was erased | e2e end; `panic wipe: a key-store failure says nothing was erased` |
| BLE relay | IMPLEMENTED on both platforms; **BLOCKED** for validation (no devices; the simulator has no Bluetooth) | Doc 21 §7 |

## Wire and compatibility criteria

| AC | Status | Evidence |
|---|---|---|
| AC-W.1 largest content fits 2048 as pre-key | TESTED | vl-crypto `largest_content_text_fits_as_pre_key_message` |
| AC-W.2 unknown ctype acknowledged, not shown | TESTED | E `unknown_content_type_is_acknowledged_and_ignored` |
| AC-W.3 malformed content rejected | TESTED | P `rejects_malformed_bytes`, `rejects_out_of_range_fields`; reject vectors |
| AC-W.4 legacy TEXT still decodes | TESTED | `receive_olm` handles `InnerKind::Text`; vl-crypto `build_text` tests |
| AC-W.5 iPhone ↔ Android over a relay hop | BLOCKED | No hardware |
| G-2 decoders fuzzed | TESTED | After groups + algorithm changes (600 s each): `decode_all` 169,698,471 runs, `link_session` 17,821,928 runs, 0 crashes. Earlier: 182.9 M / 19.6 M |

## Supporting engineering tasks

| ID | Task | Status | Evidence |
|---|---|---|---|
| A-1 | Disable SQLCipher device logging; zeroize key hex | TESTED | `sqlcipher_device_logging_is_off` (found with REA) |
| A-2 | Reject all low-order X25519 keys in cards | TESTED | `low_order_keys_in_a_signed_card_are_rejected` |
| A-3 | Measure SUMMARY airtime share in the simulator | TESTED | Simulator E2/E3: SUMMARY = 30–67% of link bytes; 13–54% of entries redundant (algorithms/03 §4) |
| A-4 | Test databases out of the system temp dir | TESTED | `CARGO_TARGET_TMPDIR`; Swift `atexit` cleanup |
| A-5 | O(1) message lookups replace full-conversation scans on receive | IMPLEMENTED | `ConversationDb::message`; covered by e2e tests (no benchmark yet) |
| A-6 | Schema v1 → v2 migration | TESTED | `version_1_database_migrates_with_data_intact` |
| A-7 | Indexed vaccine lookup (`Store::vaccine_holders`) | TESTED | 3,808.8 → 15.6 ms at 5,000 envelopes; conformance test |
| A-8 | Delta refresh | TESTED | −1.7 … −12.7% link bytes, delivery/ack unchanged (E4) |
| A-9 | Late authenticated proof upgrades Expired/DeliveryUnknown | TESTED | `receipt_arriving_after_sender_expiry_still_proves_delivery` |
| A-10 | Hybrid logical clock (ADR-025) | TESTED | `timer_change_made_after_seeing_a_skewed_change_wins`, `hybrid_clock_is_bounded_and_monotonic` |
| A-11 | Compact SUMMARY entries (link v2) | RESEARCHING | Estimated −17 … −30% link bytes; needs an ADR (algorithms/03 §6) |
