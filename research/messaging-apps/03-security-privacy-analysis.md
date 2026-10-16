# 03 — Security and Privacy Analysis

**Accessed:** 2026-10-15.

## 1. Lessons from published attacks (all REPORTED from papers or vendor advisories unless marked)

| Incident | Root cause | VYRO LINK control | Status |
|---|---|---|---|
| Bridgefy 2021/2022 (USENIX '22) | Unauthenticated broadcast; MITM-able first contact; crafted-message DoS; libsignal composition flaw | No user broadcast; in-person pairing; authenticated Noise links; bounded decoders + fuzzing | **Controls in place** (fuzz 0 crashes, Doc 20) |
| bitchat July 2025 | Identity not bound to session | Olm identity bound to the signed card; safety numbers | In place |
| Delta Chat (USENIX '24) | Cross-protocol interactions, unclear specs | Single normative wire spec (Doc 21) + golden vectors for every kind | **Must extend to CONTENT kinds** (this phase) |
| Threema (ETH 2023) | Replay state (nonce DB) lost on device migration | Seen-set and vaccines live in the Class R DB; no device migration in the MVP | Documented risk for future backup/restore |
| LINE Letter Sealing | Stateless E2EE → replay and reordering | Olm ratchet + envelope dedup; control events must be **idempotent** | Design requirement (08 AC-x) |
| WeChat MMTLS | Modified TLS; deterministic IVs; no FS | Unmodified Noise/Olm/ChaCha20-Poly1305 | In place |
| Meshtastic CVE-2025-53627 | Silent fallback to weaker encryption | No unencrypted or legacy path; unknown kinds are rejected, not downgraded | In place |
| vodozemac February 2026 report | No all-zero DH output check | **VERIFIED fixed in vodozemac 0.11.1** (contributory checks). **VYRO LINK gap fixed 2026-10-15:** `verify_card` now rejects all low-order points | **Fixed + tested** |
| SQLCipher default logging (found with REA 2026-10-15) | WARN-level logs to the public device log (os_log/logcat), including key/HMAC failure events | `PRAGMA cipher_log_level = NONE` on every open; key hex zeroized | **Fixed + tested** |

## 2. Metadata comparison

| Metadata | Mainstream (server) | bitchat | **VYRO LINK** |
|---|---|---|---|
| Who talks to whom | Server sees it (sealed sender reduces this in Signal) | Recipient tag computable by anyone who knows the recipient's static public key (announced publicly) → linkable per day (INFERRED from source) | Pairwise **secret** tag, rotating every 6 h; unlinkable to non-participants |
| Stable device identifier on air | n/a | Stable 8-byte peer ID (rotation designed, not shipped) | None; only the service UUID is advertised |
| Message size | Padded variably | Variable | **4 fixed buckets** |
| Message type | Hidden in E2EE | Hidden inside Noise | Hidden: kind is inside the wrap; all CONTENT subtypes are inside Olm |

## 3. Feature-level security implications

| Feature | Risk | Mitigation (required by 08) |
|---|---|---|
| Edit | Forged edits by others; history rewriting | Sender-only (authenticated by Olm session + contact binding); edit window 24 h of sender time; ≤ 10 edits; "edited" marker always shown |
| Delete for everyone | Forged deletes; false expectation of erasure | Sender-only; UI wording "**requested** deletion"; recipient keeps a placeholder; no claim the recipient has erased screenshots or copies |
| Reactions | Bloat; tracking via emoji choice | Fixed 8-entry set (SimpleX evidence); 1 byte per reaction |
| Disappearing messages | False sense of security | Wording: "deleted from this device after…"; no screenshot claims (iOS cannot block them; Android `FLAG_SECURE` is best-effort) |
| Search index | A plaintext index on disk | The FTS5 index lives **inside the Class C SQLCipher DB** (encrypted pages); `secure_delete` on; index rows removed with the message |
| Blocking | Blocked sender learns they are blocked | No vaccine or receipt for blocked contacts' messages (they see "Accepted by relay", then "Delivery unknown"), matching mainstream semantics |
| Drafts | Plaintext at rest | Class C DB only |
| Replies/quotes | Leaking a deleted message via its quote | A quote carries only the **target ID**, never copied text; a deleted target renders as "deleted" |
| Forwarding | Origin metadata leak | Forward = a new message with a "forwarded" flag only; no origin ID |
| Groups (Tier 2) | Member enumeration; key compromise | Pairwise fan-out reuses existing Olm sessions; the roster is never sent in clear |

## 4. New risks (proposed for Doc 10)

| ID | Risk | Likelihood / impact | Mitigation |
|---|---|---|---|
| R-NEW-1 (Doc 10 R-33) | **Harvest-now-decrypt-later**: long-lived envelopes are carried by strangers; Olm and the wrap are classical X25519 | Medium / High (long term) | Deferred: adopt a vetted hybrid PQ KEM (e.g. via a reviewed library) when one fits our constraints; **no custom design** |
| R-NEW-2 (Doc 10 R-34) | Upstream vodozemac advisories | Medium / High | Track advisories; cargo-audit/deny in CI; pin exact version |
| R-NEW-3 (Doc 10 R-35) | Control-event amplification (many edits/reactions) | Medium / Medium | Count/size caps; control events use the same envelope quotas |
| R-NEW-4 (Doc 10 R-36) | Orphan control events (target never arrives) | High / Low | Orphan rows expire after 7 d + 2 h; per-contact cap |
