# Document 20 — Development Progress

**Last updated:** 2026-10-16 · **Setup and commands:** [`../SETUP.md`](../SETUP.md) · **Wire spec:** [21](21-wire-protocol-v1.md) · **Simulation:** [22](22-routing-simulation-results.md)

> Every result below comes from a local run on the development Mac (Apple M2 Pro, macOS 26.6.2).
> - **No physical-device result exists.** Bluetooth, background behaviour, battery and cross-platform relaying are **NOT TESTED**.
> - Simulation numbers are model outputs, **not measurements**.

## 1. Phase status (Doc 18 roadmap)

| Phase | Scope | Status | Evidence |
|---|---|---|---|
| 0 | Environment | **Done**, except the Android SDK, which is waiting for you to accept its licences | `SETUP.md` §2, Doc 19 §8–§10 |
| 1 | Workspace, lints, dependency policy, CI definition | **Done**. CI is written; first run on GitHub after the 2026-10-16 push (result not yet recorded here) | `core/Cargo.toml`, `core/deny.toml`, `.github/workflows/core.yml` |
| 2a | Wire protocol: codecs, framing, vectors, fuzzing | **Done** | `vl-proto`, Doc 21, 3 fuzz targets |
| 3 | Crypto: pairing, tags and wrap, Olm, Noise links | **Done** (core); no external review | `vl-crypto` |
| 4 | Storage (SQLCipher) + routing engine + node API | **Done** (core); limitations in §5 | `vl-store`, `vl-routing`, `vl-core` |
| 5 | BLE feasibility probes (iOS/Android) | **BLOCKED**: no Android device; the iPhone is offline | §6 |
| 6 | Bindings (UniFFI) | **Swift: done and tested** (interop + used by the iOS app). Kotlin: generated, **not compiled** (no Android SDK) | `vl-ffi`, `tools/swift-interop-test.sh` |
| 7 | Native hosts (Swift/Kotlin) | **iOS: IMPLEMENTED, compiles, runs on the simulator.** Keychain keys, panic wipe, node API, BLE relay (Core Bluetooth, dual role), QR scanner and notifications are in. BLE is **not device-validated**: the simulator has no Bluetooth. **Android: IMPLEMENTED, compiles, runs on an Android 37 emulator; end-to-end test passed (1 run, 67 s)** | §2.5 |
| 8 | Flutter UI | **IMPLEMENTED and TESTED on the iOS simulator**: 21 Flutter unit/widget tests plus an end-to-end test against the real core. Authorised by you on 2026-10-15; device-dependent claims still follow Doc 14 | §2.5 |
| 9 | Security review, hardening | Not started | — |
| 10 | Beta | Not started | — |
| — | **Messaging-ecosystem research + conversation features (Stage A)** | **Research done** (25 app reports + 9 analyses). **Tier 1 features TESTED in the core**: replies, reactions, edit, delete, search, drafts, blocking, disappearing messages, pins, forwarding. **No UI**, no device validation | `research/messaging-apps/`, §2.3 |

## 2. Completed work

### 2.1 Crates (`core/crates`, Rust 2024 edition, `unsafe_code = forbid` except the FFI crate)

| Crate | Lines | What it does |
|---|---|---|
| `vl-proto` | 1,677 | 62-byte envelope header; 4 body buckets; link messages 0x01–0x09; inner message; pairing card; bounded `Reader`; length-prefixed framing with a poisoning `Deframer`. No dependencies |
| `vl-crypto` | 1,171 | Signed QR pairing cards (Ed25519 via vodozemac); X25519 + HKDF pairwise keys; 6-hour HMAC tags; ChaCha20-Poly1305 wrap with AAD; Olm v1 sessions; `Noise_NN_25519_ChaChaPoly_SHA256` links; safety numbers |
| `vl-routing` | 1,105 | Direct / epidemic / binary spray-and-wait; admission rules; quotas; vaccines; round-robin offers; sender re-spray at 25% of lifetime; `Store` trait + `MemStore` |
| `vl-store` | 1,060 | SQLCipher (rusqlite); relay DB (Class R) implementing `Store`; conversation DB (Class C) with a monotonic delivery-state machine |
| `vl-core` | 937 | `Node`: identity, pairing, send, receive, receipts, maintenance, link sessions (Noise + Deframer + routing) |
| `vl-ffi` | 276 | UniFFI surface (`VlNode`): a pull-based byte API with no foreign callbacks |
| `vl-sim` | 612 | Discrete-event routing simulator that drives the **real** engine through the wire codec |
| `uniffi-bindgen` | 3 | Host-only binding generator, kept out of the mobile library |

### 2.3 Messaging research and conversation features (2026-10-15)

- **Research** (`research/messaging-apps/`):
  - 25 per-app reports with VERIFIED / REPORTED / INFERRED / UNKNOWN labels and dated sources;
  - source inspection at pinned commits of bitchat (iOS + Android), Delta Chat core, SimpleX, Briar and Meshtastic;
  - cross-app documents 01–09: matrix, architecture, security, offline, algorithms, tiers, roadmap, acceptance criteria, tracker.
- **REA** (`reverse-engineer-anything`, CLI `rea-agents@6.5.0`) was used on **our own** artifacts:
  - `inspect-macho` on the iOS `libvl_ffi.dylib`: no networking imports, before and after the feature work (evidence IDs `ev_b720c6a7…`, `ev_bc0146c3…`);
  - `inspect-sqlite-database` on our databases: "file is not a database", i.e. encrypted at rest.
  - Proprietary apps were **not** reverse-engineered (licence terms; lawful-methods constraint).
- **Security fixes found during the work:**
  - SQLCipher's default WARN logging to the public device log is now off on every open, and the key hex is zeroized (found via REA);
  - `verify_card` now rejects **all** low-order X25519 points, not only all-zero (prompted by the February 2026 vodozemac report; vodozemac 0.11.1 itself was verified fixed in source).
- **Wire:** new inner kind `0x11 CONTENT` (ADR-023, Doc 21 §4.1), with no change to any existing byte. 9 golden vectors were added; the existing vectors are unchanged.
- **Storage:** Class C schema v2 adds reactions, edits, tombstones, drafts, conversation settings, an FTS5 index (inside SQLCipher), blocked/pinned/starred flags and sender timestamps. A v1 → v2 migration test is included.
- **Core API** (`vl-core`, exposed through `vl-ffi`):
  - `send_message` (reply/forward), `react`, `reactions`, `edit_message`, `delete_for_everyone`, `delete_for_me`;
  - `set_disappearing_timer`, `mark_read`, `search`, `save_draft`/`draft`, `set_blocked`, `set_pinned`/`set_starred`/`flagged`, `message`.
  - Receipt handling now uses O(1) message lookups instead of scanning the whole conversation.
- **Panic wipe (F-16, after ADR-023/024 approval):**
  - `Node::panic_wipe` / `VlNode.panic_wipe()` closes both databases, drops in-memory keys (the core's copy of the conversation key is now `Zeroizing`) and deletes the DB, WAL, SHM and journal files;
  - the FFI object then returns `Wiped` for every call.
  - The design is **cryptographic erasure**: the native host must delete the Keychain/Keystore keys first. That part, and the trigger UI, are not built.
- **Status per feature:** `research/messaging-apps/09-feature-implementation-tracker.md`.

### 2.4 Algorithm phase and small groups (2026-10-15)

- **Research:** `research/algorithms/` (documents 00–10):
  - 45 algorithms catalogued with sources and verdicts;
  - a comparison matrix;
  - routing, sync/conflict (normative), DB, BLE, security and architecture reports;
  - a roadmap;
  - an attachment design (not implemented).
- **Kept** (each with a baseline and an identical after-measurement, or a test that fails without the fix):
  1. **Indexed vaccine lookup:** VACCINE handling at 5,000 stored envelopes **3,808.8 → 15.6 ms** (244×).
  2. **Delta refresh:** link bytes −1.7 … −12.7%, delivery and ack unchanged (simulator E4).
  3. **Late authenticated proof** upgrades `Expired`/`DeliveryUnknown` to Delivered (bug fix).
  4. **Hybrid logical clock** (ADR-025) fixes timer causality under clock skew (bug fix), bounded to 1 h of absorbed drift.
- **Measured, no change needed:** the conversation DB (220 µs insert with FTS and fsync, 2.1 ms search at 10 k messages).
- **Hypothesis refuted:** receipt lifetime is not the cause of the weak sparse ack ratio (E1). The simulator now mirrors production anyway.
- **Biggest remaining cost found:** SUMMARY messages are 30–67% of link bytes; 13–54% of entries are already known to the peer. The proposed fix, compact entries (link v2), needs an ADR.
- **Small groups (F-13, ADR-024):**
  - content types `GROUP_STATE` / `GROUP_TEXT` / `GROUP_LEAVE` (Doc 21 §4.2);
  - creator-only roster with strictly increasing epochs;
  - messages that arrive before their roster are held (bounded);
  - removals are enforced;
  - leave requests go to the creator;
  - one pairwise copy per paired member, with delivery counts;
  - exposed through FFI (`create_group`, `set_group_members`, `leave_group`, `send_group_message`, `groups`, `group_messages`, `group_delivery`).

### 2.5 Mobile app (2026-10-15)

**Architecture** (ADR-013/022 as approved):

```
Flutter UI → Pigeon (typed) → vl_host plugin (Swift / Kotlin) → UniFFI → Rust core
```

- The UI never holds keys, parses envelopes or touches Bluetooth.
- The native hosts own the node. They pass link bytes to and from Rust; there are no Rust → native callbacks.
- Background Bluetooth work never needs Flutter.

| Area | What exists | Evidence |
|---|---|---|
| Core additions | <ul><li>Chat-list summaries, pending queue, delivery timeline, encrypted settings, relay counters, contact fingerprint and message contact id</li><li>QR pairing code (`VL1:` + Base45, RFC 9285) with self-scan refusal</li><li>QR matrix (`qrcode` 0.14.1, no default features)</li><li>Keyless database reset (`remove_database_files`)</li></ul> | `vl-core/tests/app_api.rs` (5); `vl-proto` pairing-code tests (3, RFC vectors); decode_all fuzz 600 s: **57,117,608 runs, 0 crashes** |
| Flutter app (`app/lib`) | <ul><li>Design system: Doc 16 tokens, light/dark, system fonts</li><li>Onboarding (3 pages) → identity → contextual Bluetooth request</li><li>Chats (1:1 + groups), contacts, contact profile with safety number and verification, QR pairing (show/scan), conversation (reply, react, edit, forward, pin, star, delete for me/everyone, disappearing timer, drafts, pagination, byte counter, first-message notice), delivery details timeline, groups (create, members, leave, delivered n of m), search (FTS, highlighted), network, pending queue, settings, about/security model, panic wipe</li><li>Developer tools (debug only)</li></ul> | `flutter analyze`: 0 issues. `flutter test`: **21/21**. These include accessibility guideline checks (tap targets 44/48, labels, contrast) in both themes and 200 % text |
| iOS host (Swift) | <ul><li>Keychain keys (`AfterFirstUnlockThisDeviceOnly`), destroyed and verified before files are removed</li><li>Node facade</li><li>Dual-role Core Bluetooth relay with state restoration</li><li>AVFoundation QR scanner</li><li>Generic notifications</li><li>Debug-only loopback peer</li></ul> | Builds for the simulator (`flutter build ios --simulator`). The end-to-end test `integration_test/app_flow_test.dart` **passed twice (40 s, 42 s) via `tools/run-ios-e2e.sh`** on the iPhone 18 Pro simulator (iOS 27) against the real core: onboarding → pair → send (Waiting) → exchange → **Delivered via authenticated receipt** + incoming reply → reaction → delivery timeline → panic wipe → onboarding. It found and fixed 2 app bugs (duplicate hero tags, restart after wipe) |
| Android host (Kotlin) | Keystore-wrapped keys, node facade, dual-role BLE relay (filtered scan, advertise UUID only, GATT server, MTU 517, write/notify pacing), CameraX + ZXing QR scanner, permissions, notifications, FLAG_SECURE in release, no backup | Compiled on the first try (2026-10-15, AGP 9.1, Kotlin 2.4, targetSdk 37, minSdk 31). The end-to-end test **passed** on the Android 37 arm64 emulator (`flutter drive`, 13 screenshots): Keystore-wrapped keys, node, pairing, Delivered via receipt, reaction, panic wipe → onboarding. BLE and the camera are not exercised on the emulator |
| Assets | App icon (V-shaped relay path) for iOS and Android adaptive icons; theme-matched launch screens | `tools/generate_icons.py` |

**Bugs found by the new tests and fixed:**
1. Two floating buttons with the same Hero tag broke navigation.
2. After a successful panic wipe the UI could not return to onboarding (`RootGate.restart` lookup failed from a pushed route).
3. The 44 pt network chip was below Android's 48 dp target.
4. The hidden "jump to newest" button stayed visible to screen readers.
5. Opening a chat could loop on mark-read (now `markRead` returns a count).

### 2.6 Missing features from the checklist (2026-10-16)

Built after the checklist review. Tier 3 was skipped by decision. Decisions are ADR-026, ADR-027 and ADR-028 in Doc 18 Addendum D; wire changes are in Doc 21 §4.3.

| Feature | Where | Evidence |
|---|---|---|
| My ID: 32-character Crockford Base32 code from the identity key, plus its QR; contacts show the same code | core `id_code`, My ID screen, contact profile | R (golden value), W, E2E on both platforms |
| Attachments: images ≤ 16 KiB, files ≤ 64 KiB (ADR-026: key in an Olm message, parts sealed with ChaCha20-Poly1305 as inner kind 0x12, SHA-256 check, per-part delivery proof) | core, pickers, share sheet, bubbles | R (direct, through a relay, tamper, quota), W |
| Re-pairing refreshes a contact's keys (ADR-028) | core, contact profile | R |
| Rename / delete contact | core, contact profile | R, W |
| Send again for Expired / Unknown (same message ID) | core, delivery details | R |
| Storage cap for others' mail (1–100 MB) | core, Relay settings | R, E2E |
| Android relay modes Off / While charging / Always (`connectedDevice` foreground service) | Kotlin host | E2E (screen); background relaying NOT TESTED |
| App lock (LocalAuthentication / BiometricPrompt), screenshot-block toggle (Android) | hosts, Privacy screen | W, E2E (screen); unlock prompt NOT TESTED |
| Scrubbed diagnostics export, anonymous nearby bucket | Network screen | W |
| List–detail chats layout ≥ 840 dp | Flutter | Code |

**Defects found and fixed while testing:**
1. Out-of-order attachment parts as Olm messages could exceed vodozemac's 40 skipped keys, which lost the announcement in about 1 run in 3. Fixed by the ADR-026 redesign; 12/12 repeat runs then passed.
2. Android crash `ForegroundServiceDidNotStartInTimeException` when Always mode was saved but Bluetooth permission was missing. The service now starts only with permission and while the app is in front, and stops itself if `startForeground` fails.
3. iOS build error: a main-actor isolation violation in the app-lock check.

### 2.2 Other files

- **Fuzzing:** `core/fuzz/` (targets `decode_all`, `deframer`, `link_session`).
- **Golden vectors:** `core/vectors/proto-v1.txt` (20 vectors).
- **Swift interop:** `core/bindings-tests/swift/main.swift` (11 checks), `tools/swift-interop-test.sh`.
- **CI:** `.github/workflows/core.yml`.
- **Docs:**
  - `SETUP.md`; this document; Doc 21 (wire protocol, normative); Doc 22 (simulation, generated).
  - Doc 19 completed (§4–§13).
  - Updated `00-INDEX.md`, Doc 18 (ADR-020…022 addendum) and `README.md`.
- **Git:** `git init -b main` was run with a `.gitignore` (references, skills, build outputs, fuzz corpora). **No commits** have been made.

## 3. Test results (local, 2026-10-15)

| Check | Result |
|---|---|
| `cargo test` (2026-10-15, after the app's core additions) | **119 passed, 0 failed** (adds 3 pairing-code tests and 5 `app_api.rs` tests) |
| `flutter analyze` / `flutter test` (app) | **0 issues / 21 passed** (unit, widget and accessibility guidelines in light and dark, 200 % text) |
| iOS end-to-end (`tools/run-ios-e2e.sh`, iPhone 18 Pro simulator, iOS 27, real core) | **passed** (2 runs). The simulator has no Bluetooth; the second node is the debug loopback peer |
| Swift interop (after FFI changes) | **all checks passed** |
| `cargo deny` / `cargo audit` / `osv-scanner` (after adding `qrcode` 0.14.1) | all clean |
| `cargo test` (2026-10-15, after algorithm phase + groups) | **111 passed, 0 failed**: vl-proto 23 unit + 6 property + 2 vector; vl-crypto 23; vl-routing 9; vl-store 2 unit + 13 feature + 7 store; vl-core 10 feature + 7 group + 6 three-device; vl-sim 3 |
| `cargo test` (after panic wipe) | **98 passed, 0 failed** (adds `panic_wipe_removes_databases_and_identity`) |
| `cargo test` (after Tier 1 features) | **97 passed, 0 failed**. vl-proto 22 unit + 6 property + 2 vector; vl-crypto 22; vl-routing 9; vl-store 2 unit + 12 feature + 7 store; vl-core 6 feature e2e + 6 three-device e2e; vl-sim 3 |
| `cargo test` (earlier the same day) | 72 passed, 0 failed |
| Property tests | 4,096 cases per property (proptest), regressions persisted |
| `cargo fmt --check` | clean |
| `cargo clippy --all-targets` | **0 warnings**: workspace lints deny `unwrap_used` and `indexing_slicing` outside tests |
| `cargo deny check` | advisories, bans, licences and sources all OK |
| `cargo audit` | exit 0, no advisories |
| `osv-scanner` | no issues (171 packages) |
| Fuzzing (nightly, 120 s each) | `decode_all` 45,292,272 runs; `deframer` 9,934,709 runs; `link_session` 3,985,265 runs; **0 crashes** |
| Fuzzing (nightly, 600 s each, 2026-10-15) | `decode_all` 202,930,323 runs (peak 651 MB); `deframer` 35,204,710 runs (451 MB); `link_session` 19,051,253 runs (644 MB); **0 crashes, no artifacts** |
| Swift interop | **22/22 passed** after groups (adds group roster, group message, delivery count); 19/19 after panic wipe (16 after features; 11 before): adds reply target, reaction index → fixed set, edit applied on the recipient, search, reaction validation error |
| Fuzzing after the CONTENT kind (600 s) | `decode_all` (now including `Content::decode`) **182,901,997 runs, 0 crashes**. `link_session`: the first 600 s run ended without libFuzzer's "Done" line (exit 1, no crash artifact; the session was paused at the time). A 60 s rerun completed cleanly (2,072,114 runs); the full 600 s rerun result is in §3.1 |
| iOS cross-build | `aarch64-apple-ios` and `aarch64-apple-ios-sim` release builds succeed with `IPHONEOS_DEPLOYMENT_TARGET=17.0`. After the feature work: static library 25.9 MB, dylib 3.60 MB (unstripped). **Not run on a device** |
| Kotlin bindings | Generated (3,053 lines, including the feature API). **Not compiled or run**: BLOCKED (no Android SDK or NDK) |
| Android cross-build | **BLOCKED** (no NDK) |

### 3.2 Fuzzing after the algorithm phase and groups (2026-10-15, 600 s each)

- `decode_all` (includes the group content decoders): **169,698,471 runs, peak 603 MB, 0 crashes.**
- `link_session` (exercises the indexed vaccine path and delta refresh): **17,821,928 runs, peak 692 MB, 0 crashes.**
- No artifacts.
- Final checks: fmt clean, Clippy 0 warnings, **111/111 tests**, deny/audit/OSV clean, Swift interop 22/22, iOS device and simulator builds OK, Kotlin bindings generated (3,650 lines, not compiled: BLOCKED).
- REA `inspect-macho` on the iOS dylib (evidence `ev_f5fdc2a3413dd4b2086…`): the same 4 system dependencies, **no networking imports**.

### 3.1 Fuzz rerun after the CONTENT kind (2026-10-15)

`link_session`, 600 s: **Done, 19,606,034 runs, peak 623 MB, 0 crashes, no artifacts.** This replaces the interrupted first attempt noted above. `decode_all`, 600 s, which includes `Content::decode`: 182,901,997 runs, 0 crashes.

**What the end-to-end tests prove** (`vl-core/tests/three_devices.rs`, all in-process on one Mac):
- Alice → Bob → Charlie relay with encrypted SQLCipher databases.
- Process restarts between hops.
- Link bytes fed in 20-byte and 185-byte chunks (simulated MTUs).
- Receipt or vaccine returns to Alice → Delivered.
- Olm ratchet continuation; honest expiry; malformed-byte rejection; pairing errors.

They **do not** prove anything about Bluetooth.

### 3.3 After the missing-features build (2026-10-16)

| Check | Result |
|---|---|
| Rust tests | 137 pass; clippy and rustfmt clean |
| Fuzz `decode_all` | 44,817,325 runs in 600 s, 0 crashes |
| Flutter analyze / tests | No issues / 29 pass |
| E2E iOS 27 simulator (`tools/run-ios-e2e.sh`) | Passed |
| E2E Android 37 emulator (`flutter drive`) | Passed, 16 screenshots |

## 4. Decisions made during development (with evidence)

**ADR-020…022 were approved by the user on 2026-10-15** (Doc 18 Addendum A) and are now frozen like ADR-001…019.

| ID | Decision | Evidence |
|---|---|---|
| **ADR-020** | A vaccine secret whose hash matches `ack_commitment` counts as delivery proof, like a verified receipt | Only the recipient (who opened the wrap) and the sender know `ack_secret`. A relay cannot forge it. Without this, routed receipts are often purged by vaccines before arriving (simulation, Doc 22) |
| **ADR-021** | Default spray copy budget **L = 8** (was 4) | Doc 22: campus delivery 0.90 → 0.99; sparse 0.68 → 0.79; dense 0.70 → 0.88; black-hole 0.83 → 0.94, at moderate transfer cost. Epidemic routing costs 3–6× the transfers |
| **ADR-022** | Pull-based FFI: `link_receive(bytes) → bytes`; Rust never calls Swift/Kotlin | Removes foreign callbacks and re-entrancy across the FFI boundary; the native BLE layer owns all I/O and threading |
| — | Relays **reject when full** and never evict accepted custody | Eviction would break the custody promise and let floods push out real traffic |
| — | Relays refuse **unsolicited** envelopes (only REQUESTed ones are accepted) | Stops pushing and flooding (Doc 21 §2 rule 7) |
| — | A vaccine is stored only if it matches an envelope held | Otherwise any peer can fill storage with fake vaccines |
| — | The SUMMARY `delivered` list is reserved and empty; commitments never purge | A commitment without its secret can be replayed by anyone to purge victims' envelopes |
| — | Store errors close the session (`BYE 2`) instead of being ignored | Prevents silent data loss; `Store` methods return `Result` |
| — | snow is built **without** `std`, so `ring` is not linked | `std` pulled in `ring`, which broke the iOS link and duplicated crypto |
| — | `uniffi-bindgen` is a separate crate | The `cli` feature linked clap/toml into the mobile library |
| — | MSRV is 1.89 | crates.io vodozemac 0.11.1 declares `rust-version = "1.89"` (Doc 19 §3.3 cited 1.96 for the git head) |
| — | Text limit 1,700 UTF-8 bytes | Measured maximum that fits the 2,048 bucket as a pre-key message: 1,804 bytes |

## 5. Known defects and limitations

1. **Receipts return weakly in sparse networks.** In simulation the ack ratio is 0.52 against a delivery ratio of 0.79. The sender may show "Accepted by relay" long after delivery. Options:
   - give receipts their own copy budget;
   - let the recipient carry its receipt at a higher priority;
   - accept this and rely on vaccines.

   **Open.**
2. `RECEIPT_UNDECRYPTABLE` marks the message Failed. There is no automatic re-send with a new pre-key session.
3. The Class R / Class C lock split (Doc 18 §11) is **not** enforced in `vl-core`: one `Node` opens both databases, and there is no PendingInbound queue for envelopes that arrive while Class C is locked. The native hosts must do this, and it is not yet designed in code.
4. KEY_UPDATE, REVOKE and PROFILE (0x30, 0x31, 0x40) are reserved but not implemented.
5. **vodozemac keeps only 40 skipped message keys** (max gap 2,000). Heavy reordering beyond 40 out-of-order messages loses messages. Order matters on multi-path delivery.
6. When one-time keys run out (≤ 8 per card), re-pairing in person is required. There is no key-refresh path yet.
7. The outer wrap has **no forward secrecy**: it uses static pairwise keys, while Olm provides it for TEXT. Receipts (wrap-only) are not forward-secret. This is by design, documented in Doc 06.
8. There are duplicate RustCrypto versions through snow's dependencies. This is warn-only in cargo-deny and increases binary size.
9. The v1 Olm session config uses an 8-byte truncated MAC. This is mitigated by the outer AEAD.
10. The routing simulator uses a synthetic mobility model and an assumed goodput of 20 kB/s. Its outputs **must not** be quoted as BLE performance.
11. Benchmarks exist only on the development Mac (`research/algorithms/05`). There are no on-device database or BLE throughput measurements.
12. **Groups:** no group reactions/edits/deletes; no transcript consistency (a member can equivocate, as with Signal); a message sent before a removal but arriving after it is dropped; only members you have paired with can exchange messages with you.
13. **SUMMARY overhead:** 30–67% of link bytes in simulation; compact entries are proposed (research/algorithms/03 §6).
14. **Search** uses the FTS5 `unicode61` tokenizer: word-prefix matching, weak for CJK. Messages stored under schema v1 are not search-indexed; no v1 databases exist outside tests.
15. **Disappearing messages:** the incoming timer starts only when the UI calls `mark_read`; an unread message never expires. No screenshot protection is claimed.
16. **Delete for everyone** is a *request*. It cannot recall what the recipient already read or copied, and it may never arrive (envelope expiry).
17. **Mail sent while a contact was blocked** stays undelivered after unblocking (mainstream semantics; AC-08.4).
18. No Flutter UI exists, so every feature is reachable only through the core and the bindings. The UI-wording acceptance criteria (AC-05.6, AC-09.5) are open.
19. Conversation `contact()` lookups scan the contact list (O(contacts)); fine at MVP scale.

## 6. Blocked experiments

| Item | Blocked by | Prerequisite to unblock |
|---|---|---|
| Phase 5 iOS BLE probe | iPhone17,4 is offline; Developer Mode is not confirmed | Connect and unlock the iPhone; enable Developer Mode; trust this Mac |
| Phase 5 Android BLE probe | No Android device; no Android SDK | Accept the SDK licences (SETUP §7) and obtain an Android 16/17 phone |
| **Experiment A** (iPhone ↔ Android in 10 app states) | Both of the above | Both devices + probes |
| **Experiment B** (3-device multi-hop) | Same, plus a third device | 2 Android + 1 iPhone, or 2 iPhone + 1 Android |
| **Experiment C** (battery) | Devices + 24 h test windows | As above |
| Kotlin binding test, Android cross-build | Android SDK and NDK | Accept the licences, then run `sdkmanager` (SETUP §7) |

**Result status for A, B and C: NOT TESTED.** No claim about cross-platform background relaying is made.

## 7. Risks (new or changed)

| Risk | Change |
|---|---|
| R-30 (UniFFI async) | **Avoided** by the pull-based design (ADR-022); async is not needed |
| Receipt latency (§5.1) | New. Affects the delivery UX; evaluate with the Exp B data |
| vodozemac skipped-key limit (§5.5) | New. Low with the MVP's single path per message; revisit for spray copies arriving out of order |
| SQLCipher Android build (vendored OpenSSL) | Unverified until the NDK is installed |
| R-33 … R-36 (Doc 10) | New: post-quantum harvest risk; vodozemac advisories (mitigated); control-event amplification and orphan events (mitigated by caps and expiry) |

## 8. Hardware needs

- 1–2 Android phones (Android 16 or 17, BLE 5, one with Wi-Fi off for isolation tests).
- The existing iPhone (iOS 27) online, plus ideally a second iPhone for iPhone ↔ iPhone background tests.
- Optional: a BLE sniffer (nRF52840 dongle) for Exp A diagnostics.

## 9. Next steps

**App (2026-10-15):**
1. **You:** accept the Android SDK licences in the open Terminal tab (`tools/android-sdk-setup.sh`). Then I will:
   - run `tools/build-core-android.sh`;
   - compile the Kotlin host;
   - build the APK and run the end-to-end test on the Android emulator. Bluetooth is not exercised on the emulator.
2. **You:** connect and unlock the iPhone with Developer Mode on, and set a signing team in Xcode. Then the app runs on a real device. Bluetooth relaying needs a **second** phone.
3. **Attachments (F-14):** done 2026-10-16 (ADR-026, §2.6). Needs a real-phone run to prove it over Bluetooth.
4. **Built 2026-10-16 (§2.6):**
   - app lock;
   - Android "Always relay" foreground service;
   - contact rename and delete;
   - list–detail layout on tablets.
5. **Still open:** Tier 3 features (skipped by decision), automatic key replenishment / identity rotation (FR-15), advanced diagnostics section.

**Core (earlier list):**

1. **You:** accept the Android SDK licences (SETUP §7) and bring the devices online.
2. Android: install the SDK packages; `cargo ndk` build; compile and run the Kotlin bindings test.
3. Phase 5 probes: minimal native iOS and Android apps that link `vl-ffi` and exchange `link_*` bytes over GATT, with logging limited to counters.
4. Run Experiments A → B → C (Doc 14) and record the real results there.
5. In parallel with no hardware:
   - fix §5.1 (receipt return);
   - design PendingInbound and the lock split;
   - add benchmarks;
   - run longer fuzz campaigns (≥ 1 h per target; 10 min each is done);
   - next device-independent features: group fan-out (F-13, ADR-024 approved), small-image chunk spec + simulator evaluation (F-14), SUMMARY airtime measurement (A-3).
6. ADR-023 and ADR-024 were **approved by the user on 2026-10-15** (Doc 18 Addendum B).
7. Make the decisions: licence (U-1), minimum OS versions (U-5).
