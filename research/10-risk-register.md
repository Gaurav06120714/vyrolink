# Document 10 — Technical Risk Register

**Scales:**
- **Probability:** L/M/H, with confidence in brackets (low/med/high).
- **Impact:** 1 (minor) – 5 (project-ending).
- **Type:** E = engineering risk (known how to address); **Q = unresolved research question** (we do not yet know whether it is solvable).

| ID | Risk | Type | Prob. (conf.) | Impact | Evidence | Detection | Mitigation | Residual | Go/no-go implication |
|---|---|---|---|---|---|---|---|---|---|
| R-01 | iOS background execution prevents dependable relaying | **Q** | H (high) that it is *not dependable*; M (low) that it is *useless* | 5 | ~10 s wakes, throttling [S09]; force-quit = no relaunch [S11] | E2 | Wake-on-proximity pending connects [S27]; prioritised session order; user education; Android backbone | iPhones remain best-effort relays | **Gate G1** |
| R-02 | Cross-platform discovery incompatibility (background iPhone invisible to Android) | **Q** | H (high), it is documented [S15] | 4 | S10, S15 | E1 | Dual role everywhere; the iPhone initiates to Android; Android FGS advertises | Background-iPhone ↔ background-Android pairs cannot meet | G1 |
| R-03 | BLE connection limits / instability with many peers | E/Q | M (med) | 3 | iOS limits undocumented; bitchat caps at 6 [S27]; Android batches connects [S18] | E6 | Connection budget; short sessions; RSSI gating | Slower in crowds | — |
| R-04 | Insufficient device density / no relay path | **Q** | H (med) for early adoption | 5 | DTN fundamentals [S40]; adoption unknown | E9, E12 | Target dense, closed communities first (Doc 9 §1); Architecture D option | Sparse users see long or no delivery | Product gate |
| R-05 | High delivery latency | Q | H (med) | 3 | Same | E9, E12 | Honest UX; tuning L/expiry | Inherent | — |
| R-06 | Excessive battery consumption | Q | M (low) | 4 | Models deviate from measurements [S50, S51] | E5 | Duty cycling, relay opt-in, charging-only relay option | Some users disable relaying | G3 |
| R-07 | Google Play rejects FGS `connectedDevice` for phone-to-phone relay | **Q** | M (low) | 4 | Type defined for "external devices" [S19]; all types reviewed [S24] | Early Play pre-review / internal-track submission at P3 | Clear declaration + video; fallback to `PendingIntent` scans + foreground sessions; sideload/F-Droid channel | Weaker Android backbone | G1 variant |
| R-08 | OEM battery managers kill the relay service | E | H (med) | 3 | Widely reported (C) | E2-Android, E5 on aggressive OEMs | Onboarding guidance per OEM; detect & warn | Varies by OEM | — |
| R-09 | Storage exhaustion on relays | E | M (med) | 3 | Bridgefy DoS precedent [S28, S29] | E11 | Caps, quotas, expiry, no compression | Honest relays drop mail under attack | — |
| R-10 | Malicious relays (drop, delay, black-hole, budget inflation) | E | H (high) that some exist | 3 | T2/T13 | E11, E9 | Spray to multiple; retry; clamping; vaccines | Reduced delivery | — |
| R-11 | Metadata leakage (presence, tag grouping, timing) | E/Q | H (high) that some leakage remains | 3 | Doc 6 §9; bitchat §8 | E10 | No on-air identity; Noise NN; tag rotation; padding buckets | Presence observable | Communicate in product |
| R-12 | Identity spoofing / key substitution | E | L (med) with QR-only pairing | 5 | Bridgefy history | E16, security review | Mutual QR, safety numbers, signed updates only in-session | Physical deception | G4 |
| R-13 | Incompatible crypto implementations across platforms | E | M (med) if two implementations | 4 | Two-codebase pattern [S27, S27a] | Conformance test vectors | Single portable core (ADR-004); test vectors | Low with a single core | — |
| R-14 | Licence incompatibility (GPL/AGPL code, FBP commercial licence) | E | M (high) if unmanaged | 4 | S46, S47, Doc 3 §5 | Licence audit at P2 | Use Apache/MIT/BSD/Unlicense components; legal review | — | Decide product licence at P2 |
| R-15 | Dependency abandonment (small Flutter BLE plugins, Dart crypto) | E | M (med) | 3 | `bluetooth_low_energy` last push 2026-01; `cryptography` 2025-11 | Quarterly dependency review | Native adapters (ADR-004) reduce plugin reliance | — | — |
| R-16 | Excessive architectural complexity (4 languages) | E | M (med) | 3 | ADR-004 | P3 retrospective | Decide ADR-004 only after the PoC; keep adapters thin | — | — |
| R-17 | Unsupported or changed OS behaviour (e.g. TN3115 changed in iOS 26) | Q | M (high) over time | 4 | S11 revision history | Re-run E1/E2 per OS release | Version-tagged results; defensive fallbacks | Platform drift | Continuous |
| R-18 | Poor real-world delivery reliability | Q | M (low) | 5 | Unknown until E12 | E12 | Iterate routing; target communities | — | Release gate |
| R-19 | Misleading delivery indicators | E | M (med) without care | 4 | Industry precedent | E7 + survey | Doc 7 §7 states; copy-tested wording | — | Release gate |
| R-20 | Compromised endpoints expose plaintext | E (accepted) | M (med) | 4 | Inherent | — | Storage classes; optional app lock | Inherent | Disclose |
| R-21 | Wi-Fi Aware iOS↔Android interop never matures | Q | M (low) | 2 (deferred feature) | S16 | E17 | Keep as an optional adapter | — | None for MVP |
| R-22 | Flutter engine cannot run within iOS background relaunch budget | Q → **mitigated by design** | L | 3 | ADR-013 (Doc 18): background is native + Rust only | — | Flutter used only in foreground | — | Closed unless the design changes |
| R-23 | Legal/regulatory constraints (encryption export, app store rules for E2E apps) | Q | L (low) | 3 | Not researched in depth here | Legal review at P2 | Counsel | — | P2 |
| R-24 | Clock drift breaks expiry/epochs on offline devices | E | M (med) | 2 | Design inference | E15 | Skew margins; coarse epochs | — | — |
| R-25 | No Android hardware/tooling available — validation cannot start | E (operational) | H (high) | 4 | Doc 14 §4 | — | Obtain devices; user-authorised tooling install | — | Blocks P1 |
| R-26 | Android 17 deprecates legacy `connectGatt` overloads | E | M (high) | 2 | [S68] | Build warnings | Use `connectGatt(BluetoothGattConnectionSettings, …)` on API 37+ | — | — |
| R-27 | iOS 27 Core Bluetooth regressions / undocumented changes | Q | L (low) | 3 | [S74] (beta report) | Experiment A re-runs | Re-test per 27.x; Feedback reports | — | — |
| R-28 | Single available iPhone limits generalisation | E | H (high) | 3 | Doc 14 §4 | — | Second iPhone before gate GA | — | GA quality |
| R-29 | Probe deviations from spec bias security conclusions | E | M (med) | 2 | Doc 14 §5.2 | Deviation log | Security gate limited to wrap/tag layer; full crypto at P4 | — | — |
| R-30 | UniFFI async foreign-trait support insufficient for the transport callbacks | E | M (low) | 2 | [S80] (feature tracked, release status unconfirmed) | Phase 1 spike | Synchronous foreign calls + core-side queue | — | — |
| R-31 | Audit gaps: no independent audit found for snow, RustCrypto hkdf/hmac/sha2 or SQLCipher; NCC's 2020 AEAD review predates current versions | E | M (med) | 3 | [S76, S77, S83] | — | Pin versions; cargo-audit/deny; include these integrations in the P9 external review scope | Reduced after P9 | G4 |
| R-32 | MPL-2.0 (UniFFI) obligations misunderstood | E | L (med) | 2 | Doc 18 §12 | Licence review | File-level copyleft only; keep UniFFI unmodified or publish changes | — | — |
| R-33 | Harvest-now-decrypt-later: long-lived envelopes carried by strangers are protected by classical X25519 (Olm + wrap) only | T | M (high, long-term) | 3 | research/messaging-apps/03 §4; Signal SPQR, iMessage PQ3 | — | Adopt a vetted hybrid PQ KEM only via a reviewed library; no custom design | Open | — |
| R-34 | Upstream vodozemac advisories (Feb 2026 non-contributory DH report) | T | M (high) | 2 | messaging-apps/apps/element-matrix.md | Verified fixed in 0.11.1 by source; own card check hardened | Pin exact version; cargo-audit/deny in CI; re-check on upgrade | Mitigated | — |
| R-35 | Control-event amplification (many reactions/edits) | T | M (med) | 2 | messaging-apps/03 | — | Edit ≤ 10; reactions are single-valued per author; events use normal envelope quotas | Mitigated by design | — |
| R-36 | Orphan events whose target never arrives | T | H (low) | 1 | messaging-apps/03 | `orphan_events_expire_and_are_capped` | Expire after 7 d + 2 h; ≤ 512 per contact | Mitigated | — |

## Decision gates

| Gate | When | Pass condition | Fail action |
|---|---|---|---|
| **G0 Research complete** | Now | This package reviewed by the product owner | Revise package |
| **G1 Transport** | End of P1 | E1/E2: mixed BG-iPhone ↔ FGS-Android sessions viable; H3 holds | Architecture D, an Android-only backbone, or NO-GO |
| **G2 Delivery** | End of P3/P5 | E7, E8, E13 pass; E9 usable parameter region | Routing redesign |
| **G3 Energy** | End of P1/P8 | E5 cost accepted by pilot users | Restrict relay mode |
| **G4 Security** | P9 | Independent review: no open high/critical findings | Fix + re-review |
| **G5 Release** | P10 | E12 field results meet the product requirement (Doc 9 §5) | Iterate or stop |

## Unresolved research questions (ranked by how much they could change the project)

1. (R-01/R-02) Do backgrounded iPhones participate enough?
2. (R-04) What density is needed for useful delivery in target settings?
3. (R-07) Will Google Play accept Relay mode?
4. (R-06) What is the battery cost of relaying?
5. (R-22) Can the Flutter engine be avoided or used in background wakes?
