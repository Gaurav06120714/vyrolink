# Document 17 — Testing and Acceptance Specification

**Applies to:** the architecture in Doc 18. **Status:** specification; no tests exist yet.

**Legend:**
- **AUTO** = automated, runs in CI, no physical radio.
- **DEVICE** = needs physical phones (cannot be automated in CI today).
- **TBC** = the target depends on measurements or user requirements and must be confirmed. No arbitrary number is set for these.

---

## 1. Test layers

| Layer | Where | Tooling | Run |
|---|---|---|---|
| Unit (Rust core) | `core/*` crates | `cargo test`, property tests (`proptest`), `cargo-fuzz` | AUTO, every commit |
| Protocol conformance | `vl-vectors` + both native hosts | Shared JSON test vectors executed in Rust, Swift (XCTest) and Kotlin (JUnit) | AUTO |
| Crypto | `vl-crypto` | Known-answer tests (RFC 8439 vectors, Noise test vectors, vodozemac round-trips), negative tests | AUTO |
| Database | `vl-store` | Migration fixtures, crash-consistency (kill during write) tests, GC tests | AUTO |
| Simulation | `vl-sim` | Discrete-event multi-node runs of the real core with mocked links | AUTO (nightly) |
| Integration (in-process) | Core + mock `TransportDriver` | N cores in one process exchanging frames over mock links with loss/latency | AUTO |
| Native adapters | iOS/Android hosts | Unit tests with mocked Bluetooth managers; instrumented tests on device | AUTO (logic) + DEVICE |
| UI | Flutter | Widget tests, golden tests (light/dark/200 % text), accessibility checks; integration tests via `flutter-autonomous` / Patrol on devices | AUTO + DEVICE |
| Physical-device experiments | Phones | Doc 14 A–C; Doc 08 E4, E6, E10–E12, E16 | DEVICE |
| Security | Core + app | Fuzzing, fault-injection builds, external review | AUTO + manual |

## 2. Test catalogue with acceptance criteria

### 2.1 Protocol (T-PRO), all AUTO

| ID | Test | Acceptance |
|---|---|---|
| T-PRO-01 | Encode/decode every L1 message and L2 header; round-trip property test | 100 % round-trip over ≥ 10⁶ generated cases |
| T-PRO-02 | Reject: bad version/type, bad bucket length, expiry beyond local max, hop_count ≥ hop_limit, budget 0, truncated frames | All rejected with defined codes; no panic |
| T-PRO-03 | Clamp copy_budget > 8 | Clamped to 8 |
| T-PRO-04 | Fuzz L1/L2 parsers (`cargo-fuzz`) | 0 crashes/hangs in a 24 h campaign per parser before Phase 2b; continuous afterwards |
| T-PRO-05 | Cross-platform vectors: the same bytes parse identically in Rust, Swift and Kotlin hosts | Identical results for all vectors |
| T-PRO-06 | Version negotiation (HELLO): highest common link version; unknown TLV fields ignored | Pass |
| T-PRO-07 | Max text length: largest allowed text fits the 2,048 bucket; +1 byte rejected at compose | Exact boundary verified |

### 2.2 Cryptography (T-CRY), all AUTO

| ID | Test | Acceptance |
|---|---|---|
| T-CRY-01 | Identity generation uses the OS CSPRNG; keys are unique across 10⁴ generations | No duplicates; no hard-coded keys anywhere (CVE-2025-52464 lesson [S34]) |
| T-CRY-02 | RFC 8439 ChaCha20-Poly1305 KATs; HKDF RFC 5869 vectors; HMAC-SHA256 vectors | All pass |
| T-CRY-03 | Pairing: both sides derive identical directional keys and safety numbers | Pass |
| T-CRY-04 | Tampered pairing card (sig failure), self-pairing, wrong version | Rejected |
| T-CRY-05 | Safety number changes if any key in either card changes | Pass |
| T-CRY-06 | Wrap: decrypt only with the correct directional key; AAD tamper → fail | Pass |
| T-CRY-07 | Nonce uniqueness: random 96-bit nonces; counter cap 2²⁰ per direction enforces rotation | Rotation required at cap |
| T-CRY-08 | Olm: PreKey then normal messages; out-of-order delivery up to MAX_SKIP; beyond → defined failure | Pass |
| T-CRY-09 | Replay of an Olm message → rejected | Pass |
| T-CRY-10 | Noise NN handshake against official Noise test vectors (via snow); transport nonce handling | Pass |
| T-CRY-11 | KEY_UPDATE accepted only inside an authenticated session with a valid signature | Pass |
| T-CRY-12 | REVOKE flow; old identity rejected afterwards | Pass |
| T-CRY-13 | OTK exhaustion → fallback key path works; flagged in diagnostics | Pass |
| T-CRY-14 | Secrets zeroised on drop (zeroize) where the library allows; no secrets in logs (log scrubber test) | Pass |

### 2.3 Database (T-DB), all AUTO

| ID | Test | Acceptance |
|---|---|---|
| T-DB-01 | Schema creation and every migration from fixtures | Pass; Class C lossless |
| T-DB-02 | Kill the process during every write type (fault injection) → reopen consistent | No corruption, no lost committed envelopes |
| T-DB-03 | GC removes exactly the rows with expiry < now; bounded batch size | Pass |
| T-DB-04 | Carry cap enforced; eviction by earliest expiry; own outbox never evicted | Pass |
| T-DB-05 | Class R opens without the C key; Class C never opens without its key | Pass |
| T-DB-06 | Cross-DB refs contain no names/IDs beyond opaque refs | Schema lint |
| T-DB-07 | Wrong DB key → clean failure, no data loss | Pass |
| T-DB-08 | "Clear all data" destroys keys and files | DB unreadable afterwards |

### 2.4 Routing and delivery (T-RTE), AUTO via simulator / in-process integration

| ID | Test | Acceptance |
|---|---|---|
| T-RTE-01 | Direct delivery A↔C | Delivered + ACKNOWLEDGED |
| T-RTE-02 | A→B→C live chain | Pass |
| T-RTE-03 | Time-separated encounters with persistence across simulated restarts | Pass |
| T-RTE-04 | Binary spray budget arithmetic (4 → 2/2 → 1/1) and the wait phase | Exact |
| T-RTE-05 | Multi-path duplicates → one display; vaccines purge copies | Pass |
| T-RTE-06 | Relay black hole + sender retry | Delivery via the alternate path when one exists |
| T-RTE-07 | Expiry → EXPIRED vs DELIVERY_UNKNOWN classification | Exact per Doc 18 §7.4 |
| T-RTE-08 | **Invariant:** ACKNOWLEDGED only after a verified receipt. Includes a forged receipt, a `DELIVERED_LOCAL` claim by a non-recipient, and a replayed receipt | Never set falsely (property test over random scenarios) |
| T-RTE-09 | RECEIPT_UNDECRYPTABLE → auto-resend → FAILED if it fails again | Pass |
| T-RTE-10 | Quotas: per-session, per-tag, global caps under a flood of random tags | Bounded storage; honest traffic still progresses |
| T-RTE-11 | relay_class preference when several peers are connected | Always-on preferred |
| T-RTE-12 | Simulator E9 study (delivery ratio, latency, copies, storage vs parameters) | Report produced; **targets TBC** |

### 2.5 Physical-device tests (DEVICE)

Doc 14 A (A0–A6, A-L, A-ext), B (B1–B6), C (C0–C5). Doc 08 E4 (range), E6 (congestion), E10 (sniffing), E11 (malicious relay on air), E12 (pilot), E16 (QR pairing across platforms). The acceptance rules are Doc 14 §11. **All currently NOT TESTED.**

### 2.6 Background, recovery, battery (DEVICE)

Covered by Doc 14 A2–A6 and C. Additional regression checks per release:
- iOS restoration after a system kill;
- Android FGS survival with the screen off for 8 h per OEM;
- boot restart (Android `connectedDevice` from BOOT_COMPLETED: verify, D [S20]).

### 2.7 Security and abuse

| ID | Test | Type | Acceptance |
|---|---|---|---|
| T-SEC-01 | Canary plaintext never present in Class R files, logs, diagnostics exports, HCI captures | AUTO + DEVICE | 0 occurrences |
| T-SEC-02 | Fault-injection builds (tamper, replay, forged receipt, wrong recipient, malformed frames) | AUTO + DEVICE (B5) | All rejected |
| T-SEC-03 | Connect-storm / max-rate flood from one peer | DEVICE (E11) | Battery and storage stay within caps (caps TBC) |
| T-SEC-04 | Diagnostics export scrubbing | AUTO | No tags/IDs/keys/names |
| T-SEC-05 | Dependency audit (cargo-audit/deny, OSV) | AUTO | No known vulnerable versions |
| T-SEC-06 | External review findings closed | Manual (P9) | No open high/critical |

### 2.8 UI and accessibility (T-UI)

| ID | Test | Type | Acceptance |
|---|---|---|---|
| T-UI-01 | Golden tests for all 14 screens × light/dark × 100/150/200 % text | AUTO | No clipping; matches the reviewed goldens |
| T-UI-02 | Each screen's 7 states rendered (Doc 16 D7) | AUTO | All present |
| T-UI-03 | Contrast of tokens | AUTO (script) | ≥ 4.5:1 text, ≥ 3:1 UI (current values in Doc 16 §3) |
| T-UI-04 | Screen-reader labels for every interactive element and delivery glyph | AUTO lint + DEVICE (VoiceOver/TalkBack) | 100 % labelled |
| T-UI-05 | "Delivered" shown only for ACKNOWLEDGED | AUTO | Pass |
| T-UI-06 | Reduce Motion disables the pulse | AUTO | Pass |
| T-UI-07 | RTL mirroring | AUTO golden | Pass |
| T-UI-08 | Touch targets ≥ 44 pt / 48 dp | AUTO lint | Pass |
| T-UI-09 | Diagnostics screen values match core counters | AUTO | Pass |
| T-UI-13 | Permission flows: deny / allow / revoke while running | DEVICE | Correct banners and recovery |

### 2.9 Performance

| Metric | Method | Target |
|---|---|---|
| Session duration vs wake budget | Doc 14 A (P90 vs median wake) | **TBC (measured)** |
| Core per-envelope processing time | Criterion benchmarks on reference devices | Report; target TBC as a fraction of the measured session budget |
| Cold start of the core on wake (DB open + maintenance) | Instrumented timing on devices | TBC; must leave a session budget (HA2) |
| UI frame drops | Flutter DevTools on reference devices | No jank in the conversation list at the platform frame budget |
| Battery | Doc 14 C | ≤ user threshold (TBC) |

### 2.10 Release readiness

The checklist is Doc 18 §16. Each item links to evidence: test report, raw data archive, audit report.

## 3. Automated vs physical: summary

| Can be fully automated | Requires physical devices |
|---|---|
| Codecs, parsers, fuzzing, crypto KATs, pairing derivation, DB migrations/crash consistency, routing logic, invariants, simulator studies, UI goldens, contrast, scrubbing, dependency audits | BLE discovery/connection across app states, background/locked behaviour, restoration, force-quit/reboot behaviour, real multi-hop with encounters, range, congestion, battery, OEM behaviour, screen-reader usability, air-interface privacy, Play policy review |

## 4. Device and OS test matrix

| Tier | iOS devices | iOS versions | Android devices | Android versions | Used in |
|---|---|---|---|---|---|
| **Minimum (Phase 2a/2b)** | iPhone17,4 (present) + 1 older iPhone (e.g. iPhone 12–14 class) | 27.0.x (+ 26.x if available) | 1 Pixel-class + 1 Samsung | 17 (API 37) + 15/16 | Doc 14 A–C |
| **Recommended (Phase 8)** | 4 iPhones over ≥ 3 generations | 26.x, 27.x | Pixel, Samsung, an aggressive-battery OEM, a budget device | 12 (API 31, minimum), 14, 16, 17 | Full matrix |
| Accessibility | 1 iPhone + 1 Android | latest | — | latest | T-UI-04, T-UI-13 |

**Rule:** results are tagged with the exact OS build. Major OS updates trigger re-runs of A1–A3, B1 and C3 (TN3115 changed in iOS 26 [S11]).

## 5. Acceptance governance

- No test or experiment is marked passed without stored raw evidence (logs, captures, reports) and the build hash.
- TBC targets become fixed only through a documented decision (ADR or Doc 14 §11 update) after the measurements exist.
