# bitchat — research report

**Accessed:** 2026-10-15. **Source code inspected:**
- `permissionlesstech/bitchat` @ `5e9287f` (2026-09-24, iOS/macOS, Unlicense)
- `permissionlesstech/bitchat-android` @ `6ade682` (2026-10-11, GPL-3.0; studied only, nothing copied)

**Overall confidence:** High for the code paths cited below; medium for runtime behaviour, because nothing was executed.

**Labels:**
- **VERIFIED** = read in source or the project whitepaper at the pinned commit.
- **REPORTED** = credible secondary source.
- **INFERRED** = our analysis.
- **UNKNOWN** = not established.

## 1. Why it matters to VYRO LINK

bitchat is the closest existing system: a BLE dual-role mesh with offline store-and-forward. It has **independently converged on the same core mechanisms VYRO LINK chose**:
- binary spray-and-wait with an 8-copy cap;
- rotating recipient tags;
- bounded carrier quotas;
- solicited-only sync responses.

That convergence is evidence for our design (ADR-003, ADR-021). It also shows where bitchat chose differently.

## 2. Capabilities

| Capability | Status | Label | Where |
|---|---|---|---|
| Public broadcast chat (unencrypted, signed announce) | Yes | VERIFIED | `MessageType.message 0x02` |
| Private 1:1 (Noise sessions) | Yes | VERIFIED | `noiseHandshake 0x10`, `noiseEncrypted 0x11` |
| Delivery receipts / read receipts | Yes; inside Noise | VERIFIED | `NoisePayloadType.delivered 0x03`, `readReceipt 0x02` |
| Delivery states | notSentYet, sending, sent, **carried**, delivered(to, at), read, failed, partiallyDelivered (rooms) | VERIFIED | `BitFoundation/DeliveryStatus.swift` |
| Store-and-forward "courier" | Yes. Only **mutual favorites** carry (40 envelopes total, 20 for verified non-favorites, 5 per favorite depositor, 2 per verified depositor; 24 h lifetime; ≤ 16 KiB ciphertext; text only) | VERIFIED | `Services/Courier/CourierStore.swift`, `CourierEnvelope.swift` |
| Spray-and-wait copies | `maxCopies = 8`, binary split; 1 = carry-only | VERIFIED | `CourierEnvelope.swift` |
| Recipient tag | `HMAC-SHA256(recipient static key, "bitchat-courier-tag-v1" ‖ UTC day)`, truncated to 16 bytes; adjacent days are checked | VERIFIED | `CourierEnvelope.swift` |
| Forward-secret courier seal (v2) | Noise X to a gossiped one-time prekey | VERIFIED | `prekeyBundle 0x24`; seal discriminator TLV |
| Gossip sync | Golomb-coded-set (GCS) filter `REQUEST_SYNC`; unicast; responses flagged `IS_RSR`, TTL 0, rate-limited 8/30 s; unsolicited responses rejected; ±2 min timestamp window on normal packets | VERIFIED | `docs/REQUEST_SYNC_MANAGER.md` |
| Private groups | Creator-signed roster + one symmetric ChaCha20-Poly1305 group key per epoch; key rotation on roster change; group ID in cleartext for relay dedup; only the creator may sign state | VERIFIED | `Services/Groups/GroupProtocol.swift` |
| Files / voice notes / images | Yes (`fileTransfer 0x22`, `privateFile 0x20`); **not** carried by couriers | VERIFIED | `MessageType.swift`, `CourierEnvelope.maxCiphertextBytes` |
| Push-to-talk live voice bursts | Yes (`voiceFrame 0x29`); ephemeral, never synced | VERIFIED | `MessageType.swift` |
| QR verification + "vouch" web of trust | Yes (`verifyChallenge`/`verifyResponse`/`vouch`) | VERIFIED | `BitchatProtocol.swift` |
| Peer-ID rotation with pairwise recognition tags | Designed; **not shipping yet** | VERIFIED | `announceV2 0x2C` comment, `docs/PEER-ID-ROTATION.md` |
| Internet fallback | Nostr relays (over Tor optionally) for mutual favorites | VERIFIED | `WHITEPAPER.md` §2 |
| Panic wipe | Yes; includes outbox and courier store | VERIFIED | `MessageOutboxStore.swift`, `AppChromeModel.swift` |
| Persistent history | **Ephemeral by default**; the timeline lives in memory; the outbox is sealed with a Keychain key | VERIFIED | `WHITEPAPER.md` §1 |
| Reactions, edits, deletes, replies | **Not found** in payload types | VERIFIED (absence within the enums inspected) | — |

## 3. Architecture and transport (VERIFIED, `WHITEPAPER.md` v2.0, 2026-07-06)

- Two transports sit behind a `Transport` interface, coordinated by a `MessageRouter`: the BLE mesh (central + peripheral, controlled flood with TTL) and Nostr. The order of preference is: live mesh link → Nostr → courier.
- Identity is a Curve25519 Noise static key plus an Ed25519 signing key. The 8-byte mesh peer ID is derived from the static-key fingerprint, so it is **stable and linkable**. The project's own rotation design acknowledges this.

## 4. Security and privacy

- **Early 2025:** identity checks were broken, allowing a MITM to impersonate a trusted contact (REPORTED, Trail of Bits blog, 2025-07-18). The project moved to the Noise framework in July 2025 (REPORTED).
- **Audit status:** a January 2026 claim of a Cure53 audit is **UNKNOWN**; we found no report. The README warning about missing external review was present historically (REPORTED).
- **CVE-2026-51376:** DoS via an unauthenticated MESSAGE packet in the gossip cache; fixed in 1.5.1 (REPORTED, Mend.io).
- **Tag linkability (INFERRED):** the courier tag can be computed by **anyone who knows the recipient's static public key**, and those keys travel in public announces. So any observer who has seen the recipient announce can link envelopes to that recipient for a day.
  - VYRO LINK's tags use a **pairwise secret** (HKDF over X25519 of the two addressing keys), so only the sender and recipient can compute them. This is a deliberate improvement; keep it.

## 5. Offline behaviour (VERIFIED)

- Outbox retention is 24 hours.
- The courier carries only for mutual favorites or verified peers, under small quotas.
- The courier store is opaque ciphertext and is wiped on panic.

## 6. Lessons for VYRO LINK

**Adopt:**
- the **"carried" state** wording;
- small per-depositor quotas, plus a trust tier for verified contacts;
- link-local, solicited-only sync (we already do this);
- a panic wipe (Tier 2);
- the **partiallyDelivered(n/total)** state for groups.

**Evaluate:** GCS filters versus our explicit SUMMARY list (doc `05-technology-algorithm-evaluation.md` §2).

**Avoid:**
- public-key-derived tags (linkable);
- stable peer IDs;
- unauthenticated public broadcast in the MVP;
- the Nostr/internet fallback (contradicts offline-only ADR-012).

## 7. Sources

- Source code at the commits above (local clones in `references/`, git-ignored).
- https://blog.trailofbits.com/2025/07/18/building-secure-messaging-is-hard-a-nuanced-take-on-the-bitchat-security-debate/ (accessed 2026-10-15, high confidence for the 2025 issues).
- https://www.theregister.com/2025/07/08/jack_dorsey_debuts_bitchat/ (accessed 2026-10-15, medium).
- https://mend.io/vulnerability-database/CVE-2026-51376/ (accessed 2026-10-15, medium; version fields inconsistent).
- https://reclaimthenet.org/recommended-tool/bitchat (accessed 2026-10-15, low–medium).
