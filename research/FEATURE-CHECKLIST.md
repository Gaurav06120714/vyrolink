# VYRO LINK — Research vs. Built App Checklist

**Checked:** 2026-10-16 (after the missing-features build), against the apps in `app/`: Flutter UI, Swift host and Kotlin host, all on the Rust core in `core/`.

**Sources:** everything researched in this folder. That is:
- Docs 09, 16 and 18 (blueprint FR/NFR);
- `messaging-apps/` 01, 06 and 08 (25 apps, feature tiers, acceptance criteria);
- `algorithms/`;
- Doc 21 (wire protocol).

**Legend**

| Mark | Meaning |
|---|---|
| ✅ | Present in the app and wired to the real core |
| 🟡 | Partly present; the missing part is named in the note |
| ❌ | Not built |
| 🚫 | Rejected by the research on purpose (shown so the list is complete; not counted as missing) |

**Evidence codes**

| Code | Meaning |
|---|---|
| **E2E** | Proven by the end-to-end test (`integration_test/app_flow_test.dart`), which passed against the real Rust core on the iOS 27 simulator and the Android 37 emulator. Latest run on 2026-10-16 passed on both, and it now also opens Relay settings, My ID (asserts the 32-character code) and Privacy |
| **W** | Flutter widget/unit test (`app/test`, 29 passing) |
| **R** | Rust test in `core/` (137 passing; clippy and rustfmt clean; `decode_all` fuzzed 44.8 M runs, 0 crashes, after the attachment changes) |
| **Code** | Implemented, not exercised by an automated app test |

> **Nothing is DEVICE-VALIDATED.**
> - No physical phones were used. The simulator has no Bluetooth, and the emulator has no real Bluetooth radio.
> - Every Bluetooth row below means "implemented", not "proven between phones".

---

## 1. Messaging features (messaging-apps/06 tiers)

| ID | Feature | iOS | Android | Evidence | Note |
|---|---|:-:|:-:|---|---|
| F-01 | In-person QR pairing (show code, scan code) | ✅ | ✅ | R, Code | Signed card as `VL1:` Base45 QR; self-scan refused. Camera scanning is AVFoundation on iOS and CameraX + ZXing on Android; not exercised (no real camera in simulator/emulator) |
| F-01b | Safety number + "mark verified" | ✅ | ✅ | Code | 12 × 5 digits, verify sheet, badge on contact |
| F-02 | Honest delivery states (Waiting → Accepted by relay → Delivered → Expired / Unknown) | ✅ | ✅ | E2E, W, R | "Delivered" only after an authenticated receipt (ADR-015) |
| F-03 | Replies (quote by ID) | ✅ | ✅ | R, Code | Quote preview, reply bar |
| F-04 | Reactions (8 fixed) | ✅ | ✅ | E2E, R | One-row picker, 48 dp targets |
| F-05 | Delete for me / delete for everyone (72 h) | ✅ | ✅ | R, Code | Recall-limit warning in the confirm dialog |
| F-06 | Local search (encrypted FTS5) | ✅ | ✅ | R, Code | Global + per chat, highlighted matches |
| F-07 | Drafts | ✅ | ✅ | R, Code | Saved while typing, restored on open |
| F-08 | Blocking | ✅ | ✅ | W, R | Block/unblock in profile; composer replaced while blocked |
| F-09 | Disappearing messages | ✅ | ✅ | R, Code | Per-chat timer sheet, header indicator, timer icon on bubbles |
| F-10 | Edit (24 h, ≤ 10) | ✅ | ✅ | R, Code | Edit bar; "Edited" label |
| F-11 | Pins / stars (bookmarks) | ✅ | ✅ | R, Code | Pinned and starred lists per chat |
| F-12 | Forwarding | ✅ | ✅ | R, Code | "Forwarded" label, contact picker |
| F-13 | Small groups (≤ 8, pairwise copies) | ✅ | ✅ | R, Code | Create, change members (creator), leave, author names, "Delivered to n of m" |
| F-14 | Small attachments: images ≤ 16 KiB, files ≤ 64 KiB | ✅ | ✅ | R, W | ADR-026: one-time key inside the Olm message, parts sealed with ChaCha20-Poly1305 and checked by SHA-256. Images are re-encoded (no location metadata). "Delivered" only when every part is proven. Picker and share sheet not exercised by an automated test |
| F-15 | Offline notifications | ✅ | ✅ | Code | Generic "You have a new message" only, after a Bluetooth session while in the background. Not device-tested |
| F-16 | Panic wipe (keys destroyed first, then files) | ✅ | ✅ | E2E, W, R | Keychain / Keystore keys deleted and verified; UI returns to setup; failure explained |
| F-17 | Per-contact privacy preferences | ❌ | ❌ | — | Not designed yet |
| F-18 | Receipt-return improvement | ❌ | ❌ | — | Simulator research item, not an app feature yet |
| F-19 | Voice notes | ❌ | ❌ | — | Tier 3 (optional) |
| F-20 | Polls | ❌ | ❌ | — | Tier 3 |
| F-21 | Encrypted local backup | ❌ | ❌ | — | Tier 3; backups are disabled on purpose today |
| F-22 | "Paper" message via QR | ❌ | ❌ | — | Tier 3 |
| F-23 | Introductions | ❌ | ❌ | — | Tier 3 |
| F-24 | My ID: unique 32-character code + its QR (added on request) | ✅ | ✅ | E2E, W, R | ADR-027: Crockford Base32 (all digits and letters except I L O U) of a SHA-256 hash of the identity key. Unique per phone without any account; contacts see the same code to compare |

## 2. Features compared across the 25 researched apps (messaging-apps/01)

| Feature | iOS | Android | Note |
|---|:-:|:-:|---|
| 1:1 end-to-end encryption by default | ✅ | ✅ | Olm (vodozemac) + wrap; R |
| Groups | ✅ | ✅ | Small groups, F-13 |
| Reactions / replies / edit / delete-for-all / disappearing | ✅ | ✅ | §1 |
| Delivery receipts | ✅ | ✅ | Authenticated, E2E |
| Read receipts | 🚫 | 🚫 | Deferred by research (privacy; costs envelopes) |
| Voice notes | ❌ | ❌ | Tier 3 |
| Attachments | ✅ | ✅ | Small images and files, F-14 |
| Search / pins / drafts | ✅ | ✅ | §1 |
| Polls | ❌ | ❌ | Tier 3 |
| Channels / broadcast | 🚫 | 🚫 | Rejected (abuse surface) |
| No phone number / no account | ✅ | ✅ | Identity made on the phone |
| In-person QR pairing + key verification | ✅ | ✅ | §1 |
| Multi-device | 🚫 | 🚫 | Deferred by research |
| Typing indicators / online / last seen | 🚫 | 🚫 | Rejected (metadata) |
| Calls | 🚫 | 🚫 | Rejected (BLE cannot carry them) |
| Cloud sync / backup / internet fallback | 🚫 | 🚫 | Rejected (offline by design) |

## 3. Screens (Doc 16 §8)

| # | Screen | iOS | Android | Evidence | Note |
|---|---|:-:|:-:|---|---|
| 1 | Onboarding (3 pages) | ✅ | ✅ | E2E, W | Skip from page 2; identity cannot be skipped |
| 2 | Identity setup | ✅ | ✅ | E2E, W | Name + keys ✅. App lock is offered in Privacy (screen 11) |
| 3 | Home (Chats) | ✅ | ✅ | E2E, W | Empty state, unread badges, network chip, relay/permission banners |
| 4 | Contact list + profile | ✅ | ✅ | W, R | List, verify, block, timer ✅. Rename locally, delete contact (with its chat and carried mail), their ID code, "Pair again" to refresh keys ✅ |
| 5 | QR pairing (show / scan / steps / verify) | ✅ | ✅ | R, Code | Camera-permission flow and error messages included |
| 6 | Nearby (presence without identities) | ✅ | ✅ | E2E, W | Merged into Network: exchange count, last exchange, and "Phones met in the last day" as a bucket (None / 1–2 / 3–9 / 10+). Phones rotate Bluetooth addresses, so the count is approximate |
| 7 | One-to-one chat | ✅ | ✅ | E2E, W | First-message notice, byte counter, pagination, jump to newest |
| 8 | Delivery details | ✅ | ✅ | E2E, R | Timeline + "what these mean" ✅. "Send again" for Expired / Unknown (same message ID, so no duplicate) ✅. Per-part progress for files |
| 9 | Network and relay diagnostics | 🟡 | 🟡 | E2E, W | Status, counters, carried bytes ✅. Scrubbed diagnostics export (counters only; W) ✅. **Advanced section ❌** |
| 10 | Pending message queue | ✅ | ✅ | Code | Grouped by contact, opens delivery details |
| 11 | Privacy settings | ✅ | ✅ | E2E, W | App lock (Face ID / Touch ID / fingerprint, with the phone passcode as fallback; locks on open and after 30 s away). Android screenshot-block toggle. Previews always hidden. Panic wipe. The unlock prompt itself is not exercised (no passcode on the test devices) |
| 12 | Battery and relay settings | ✅ | ✅ | E2E, R | Storage cap for others' mail, 1–100 MB (R). Android: Off / While charging / Always. iOS has no such modes (the system decides background time), so it shows the cap only. Battery cost not measured |
| 13 | Permissions and troubleshooting | ✅ | ✅ | Code | Bluetooth / camera / notifications status, Open Settings, "never asks for location" |
| 14 | About and security model | ✅ | ✅ | Code | What relays see, limits, identity key, "Not yet independently reviewed", licences |
| — | Group chat + group info | ✅ | ✅ | Code | Not a Doc 16 screen; added for F-13 |
| — | Search screen | ✅ | ✅ | Code | — |
| — | Panic-wipe screen | ✅ | ✅ | E2E, W | — |
| — | Developer tools (debug builds only) | ✅ | ✅ | E2E | Loopback test peer; refused in release builds |

## 4. Functional requirements (Doc 18 §3)

| FR | Requirement | iOS | Android | Note |
|---|---|:-:|:-:|---|
| FR-01 | Identity created locally, no account | ✅ | ✅ | E2E |
| FR-02 | Mutual QR pairing, safety number, mark verified | ✅ | ✅ | Camera not exercised |
| FR-03 | Compose and send text offline (≤ 1,700 bytes) | ✅ | ✅ | E2E, W (byte limit) |
| FR-04 | End-to-end encryption; relays see no plaintext | ✅ | ✅ | R |
| FR-05 | Discover nearby devices over BLE and exchange automatically | 🟡 | 🟡 | Implemented (dual role); **not validated on phones** |
| FR-06 | Relay others' mail within user-set limits | ✅ | ✅ | User-set cap (R) + Android relay modes (E2E screen). Not device-tested |
| FR-07 | Multi-hop delivery over time-separated encounters | 🟡 | 🟡 | Core + simulator ✅ (R); **no device proof** |
| FR-08 | Authenticated receipt; "Delivered" only on proof | ✅ | ✅ | E2E, R |
| FR-09 | Duplicate suppression | ✅ | ✅ | R |
| FR-10 | Queue and carried mail survive restarts | ✅ | ✅ | R (SQLCipher persistence) |
| FR-11 | Expiry; Expired / Delivery unknown shown honestly | ✅ | ✅ | R; labels in UI |
| FR-12 | Diagnostics screen + scrubbed export | ✅ | ✅ | Screen ✅; export via the share sheet, counters only (W) |
| FR-13 | Permission education and recovery | ✅ | ✅ | Contextual requests + Settings links |
| FR-14 | Local delete of messages/conversations; crypto-shred all data | ✅ | ✅ | Message delete ✅, delete contact + conversation ✅ (R), wipe-all ✅ |
| FR-15 | One-time-key replenishment, identity rotation/revocation | ❌ | ❌ | Re-pairing in person now refreshes a contact's keys (ADR-028, R). Automatic replenishment and identity rotation are still not built |

## 5. Non-functional requirements (Doc 18 §3)

| NFR | Requirement | iOS | Android | Note |
|---|---|:-:|:-:|---|
| NFR-01 | No server dependency | ✅ | ✅ | — |
| NFR-02 | No network permission in builds | ✅ | ✅ | iOS: no network code. Android: INTERNET only in Flutter's debug/profile manifests; `ACCESS_NETWORK_STATE` (from a CameraX dependency) removed from the merged manifest |
| NFR-03 | WCAG AA contrast, 200 % text, screen-reader labels | ✅ | ✅ | W: tap-target, label and contrast guidelines in light and dark; 200 % text |
| NFR-04 | One protocol implementation for both platforms | ✅ | ✅ | Same Rust core via UniFFI |
| NFR-05 | Link session fits the iOS background wake budget | ❌ | — | Needs device measurement (Doc 14) |
| NFR-06 | Battery cost within threshold | ❌ | ❌ | Needs device measurement |
| NFR-07 | Strict bounded parsing, no decompression | ✅ | ✅ | R + fuzzing (decode_all 57 M runs, 0 crashes, after the QR codec) |
| NFR-08 | All data encrypted at rest with OS-protected keys | ✅ | ✅ | SQLCipher; Keychain (`AfterFirstUnlockThisDeviceOnly`) / Keystore-wrapped keys |
| NFR-09 | No telemetry, analytics or data-sending SDKs | ✅ | ✅ | No ML Kit (ZXing instead); no analytics packages |
| NFR-10 | Dependency audit | ✅ | ✅ | cargo-deny, cargo-audit, OSV clean |

## 6. Security and privacy (Docs 06, 18, messaging-apps/03)

| Item | iOS | Android | Note |
|---|:-:|:-:|---|
| Keys never leave the device; excluded from backups | ✅ | ✅ | iOS: ThisDeviceOnly + excluded from backup. Android: `allowBackup=false`, data-extraction rules exclude everything |
| Panic wipe = cryptographic erasure before file removal | ✅ | ✅ | E2E |
| No plaintext, keys or IDs in logs | ✅ | ✅ | Errors carry codes only; SQLCipher logging off (R) |
| Advertising carries only the service UUID (no name / ID) | ✅ | ✅ | Doc 21 §7 |
| Notifications never show sender or text | ✅ | ✅ | — |
| Screen-capture protection | 🚫 | ✅ | Android (FLAG_SECURE): on by default in release builds, user toggle in Privacy. iOS cannot block screenshots (research rejected claiming it) |
| App lock (biometric) | ✅ | ✅ | iOS LocalAuthentication; Android BiometricPrompt (biometric or device credential). Cannot be turned on without a phone passcode (W) |
| External security audit | ❌ | ❌ | Shown in the app as "Not yet independently reviewed" |
| Post-quantum hybrid | 🚫 | 🚫 | Deferred by research |

## 7. Bluetooth transport (Docs 04, 14, 18; algorithms/06)

| Item | iOS | Android | Note |
|---|:-:|:-:|---|
| Dual role (central + peripheral) GATT link | ✅ | ✅ | Code; one characteristic, write-without-response + notify |
| MTU-sized chunking with platform flow control | ✅ | ✅ | Code |
| Session end on idle / time limit, reconnect cooldown | ✅ | ✅ | Code |
| Background relaunch (iOS state restoration) | ✅ | — | Code; force-quit stops it (iOS rule, stated in the app) |
| Android foreground service "Always relay" | — | ✅ | `connectedDevice` service with a visible notification, only in Always mode. It starts only when Bluetooth permission is granted. A crash found on the emulator (service started without permission) was fixed. Background relaying is not device-tested |
| Filtered long scan (screen-off safe) | — | ✅ | Code |
| L2CAP channel upgrade | ❌ | ❌ | Optional; only if Experiment A shows a gain |
| Exponential backoff with jitter (shared in Rust) | 🟡 | 🟡 | Fixed cooldown in each host; shared scheduler not built (algorithms/09 N6) |
| **Real phone-to-phone delivery (iPhone ↔ Android, multi-hop)** | ❌ | ❌ | **Not validated: needs two physical phones** (Doc 14 A–C) |

## 8. Design and accessibility (Doc 16 §1–§7, §10–§11)

| Item | iOS | Android | Note |
|---|:-:|:-:|---|
| Design tokens (warm-grey canvas, signal-blue accent), light + dark | ✅ | ✅ | W |
| System fonts, Dynamic Type / font scale | ✅ | ✅ | W (200 %) |
| Seven states (empty, loading > 300 ms, error, offline, success, permission) | ✅ | ✅ | Code |
| Native feel (Cupertino transitions, no ripple on iOS; predictive back on Android) | ✅ | ✅ | Code |
| Reduce Motion honoured (static network dot) | ✅ | ✅ | Code |
| Haptics on send and on verification only | ✅ | ✅ | Code |
| 44 pt / 48 dp touch targets | ✅ | ✅ | W (both guidelines) |
| Screen-reader labels (glyphs announce their state) | ✅ | ✅ | W |
| RTL mirroring | 🟡 | 🟡 | Directional layout used throughout; not tested |
| Tablet: navigation rail ≥ 840 dp | ✅ | ✅ | Code |
| Tablet: list–detail layout | ✅ | ✅ | Code: chats list + open chat side by side from 840 dp |
| Localization (more than English) | ❌ | ❌ | English only |
| App icon + theme-matched launch screen | ✅ | ✅ | `tools/generate_icons.py` |

---

## 9. Score

Counted row by row over sections 1–8. 🚫 rows and platform-specific "—" rows are excluded.

| Section | iOS ✅ / 🟡 / ❌ | Android ✅ / 🟡 / ❌ |
|---|:-:|:-:|
| §1 Messaging features (25) | 18 / 0 / 7 | 18 / 0 / 7 |
| §2 Cross-app features (10 not rejected) | 8 / 0 / 2 | 8 / 0 / 2 |
| §3 Screens (18) | 17 / 1 / 0 | 17 / 1 / 0 |
| §4 Functional requirements (15) | 12 / 2 / 1 | 12 / 2 / 1 |
| §5 Non-functional requirements | 8 / 0 / 2 | 8 / 0 / 1 |
| §6 Security and privacy | 6 / 0 / 1 | 7 / 0 / 1 |
| §7 Bluetooth | 4 / 1 / 2 | 5 / 1 / 2 |
| §8 Design and accessibility (13) | 11 / 1 / 1 | 11 / 1 / 1 |
| **Total** | **84 / 5 / 16** | **86 / 5 / 15** |

Before this round: iOS 70 / 14 / 20, Android 71 / 14 / 20.

**Biggest gaps, in order of value:**
1. **Real phone-to-phone Bluetooth validation:** needs two phones. This also covers background relaying, attachments over a real link, and battery cost.
2. **Automatic key replenishment / identity rotation** in the core (FR-15). Re-pairing in person is the current way to refresh keys.
3. **Tier 3 features** (voice notes, polls, encrypted backup, paper messages, introductions): skipped for now by decision.
4. **Advanced diagnostics section, localization, shared backoff scheduler, L2CAP.**
