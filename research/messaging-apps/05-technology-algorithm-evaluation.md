# 05 — Technology and Algorithm Evaluation

**Accessed:** 2026-10-15.

**Rule:** popularity is not evidence. Each choice states its evidence and confidence, and keeps the existing core where it is sound.

## 1. Routing and bounded replication

| Option | Evidence | Verdict |
|---|---|---|
| **Binary spray-and-wait, L = 8 (current)** | Doc 22 simulation; bitchat independently chose `maxCopies = 8` with a binary split (VERIFIED, `CourierEnvelope.swift`) | **Keep** (ADR-003, ADR-021) |
| Epidemic | 3–6× the transfers in Doc 22 | Reject as default |
| PRoPHET / utility routing | Needs encounter history, which is a metadata cost | Defer; revisit with Experiment B data |
| Managed flood + next-hop (Meshtastic 2.6) | Requires stable node IDs, which conflicts with unlinkability | Reject |

## 2. Duplicate suppression and set reconciliation

| Option | Size per item | Error | Fit |
|---|---|---|---|
| **Explicit SUMMARY list (current):** 38 B per entry, ≤ 128 entries (≤ 4.9 KB) | 304 bits | Exact | Simple, bounded, fuzzed. At the simulator's assumed 20 kB/s, a full summary costs ~0.25 s per session |
| Golomb-coded set (bitchat `REQUEST_SYNC`; BIP158) | ~20 bits at a 2⁻²⁰ false-positive rate (~70% of a Bloom filter) | False positives → a missed item until the next round | Smaller, but adds a new codec and probabilistic misses |
| Bloom filter | ~29 bits at the same rate | Same | Dominated by GCS |
| Minisketch / IBLT | Scales with the *difference*, not the set; quadratic decode | Fails when capacity is exceeded | Overkill for ≤ 128 items |
| **Seen set (current):** exact DB rows expiring with envelope expiry | — | Exact | **Keep**. A probabilistic seen set would silently drop real messages |

**Decision:** keep the explicit SUMMARY. Re-evaluate **only if Experiment C shows summary bytes dominate session airtime**. The simulator can measure summary share without hardware; this is an open task (09 tracker A-3).

## 3. Expiry, retry and queue management

- **Expiry:** absolute `expiry_min` with ±30 min sender jitter (current). Keep.
- **Retry:** one sender re-spray at 25% of lifetime (current). Exponential backoff with jitter suits *connection* attempts in the native BLE layer (Phase 7), not envelope routing.
- **Queues:**
  - per-session ≤ 32 envelopes / 64 KiB; per tag ≤ 8; carry ≤ 10 MiB;
  - reject-when-full (current);
  - offers are round-robin by tag, own envelopes first (current).

  bitchat uses even smaller quotas (40 envelopes total), which suggests our 10 MiB cap is generous. Keep it configurable and decide with Experiment C.
- **Receipt forwarding (open defect):** in sparse networks the ack ratio is 0.52 against a delivery ratio of 0.79 (Doc 22). The candidates below are compared **in the simulator first** (no hardware needed):
  - (a) give receipts their own initial budget;
  - (b) prioritise receipts in offers;
  - (c) rely on vaccines (already counted as proof, ADR-020).

## 4. Event model for conversational features (new; specified in Doc 21 §4.1)

**Evidence:**
- Matrix replacement/redaction events, SimpleX `XMsg*` events and Meshtastic `reply_id` all use target references (REPORTED/VERIFIED).
- Delta Chat tombstones and sender checks: VERIFIED in `receive_imf.rs`.

**Decisions:**
- One new inner kind **`0x11 CONTENT`**, Olm-protected exactly like TEXT. Inside the Olm plaintext:

  `content_version(1) ‖ ctype(1) ‖ body`

  with ctype values TEXT, REACTION, EDIT, DELETE, TIMER.
  - **No new cryptography.** The existing wrap and Olm layers protect it.
  - The content type is invisible to relays, because it is inside the wrap and inside Olm.
- **Merge rules are order-independent** (the result is the same for any arrival order):
  - **Reaction:** per (target, reactor), the greatest `(sent_at_ms, message_id)` wins; reaction 0 removes it.
  - **Edit:** per (target), the greatest `(seq, sent_at_ms)` wins; only the target's sender; `seq` ≤ 10; sender time within 24 h of the target.
  - **Delete:** a tombstone per target; only the target's sender; within 72 h of sender time; idempotent. A late original is stored as deleted.
  - **Timer:** the conversation timer follows the greatest `sent_at_ms` among received timer values.
- **Orphans** (control arrives before its target): stored keyed by target, applied when the target arrives; expire after 7 d + 2 h (max lifetime + skew); capped per contact.

## 5. Search and local storage

| Option | Evidence | Verdict |
|---|---|---|
| **SQLite FTS5 inside the Class C SQLCipher DB** | `SQLITE_ENABLE_FTS5` is compiled into our bundled SQLCipher (VERIFIED, `libsqlite3-sys-0.38.2/build.rs`); the index pages are encrypted like all pages | **Adopt.** No new dependency |
| Tokenizer `unicode61` (default) | Word-based, diacritic folding; weak for CJK (no word boundaries) | MVP default, with prefix queries |
| Tokenizer `trigram` | Substring search, CJK-friendly; larger index; queries < 3 chars cannot match | Option if users need CJK search |
| Separate search engine (tantivy, etc.) | A new dependency and a second on-disk index outside SQLCipher | Reject |

**Query safety:** user input is turned into quoted FTS5 phrase-prefix terms (`"term"*`), so FTS syntax cannot be injected.

## 6. Group messaging

| Option | Crypto design needed | Network cost per message | Offline fit | Verdict |
|---|---|---|---|---|
| **Pairwise fan-out over existing Olm sessions** | **None** | n envelopes | Good: each copy routes like a 1:1 message | **Tier 2 MVP groups (≤ 8 members)** |
| Megolm (vodozemac `GroupSession`, audited 2022) | Key distribution over Olm exists; **a group routing tag would be new design** | 1 envelope only with a group tag; otherwise n | Good | Deferred; needs a reviewed group-tag design |
| Creator-signed roster + epoch key (bitchat) | New shared-key scheme | 1 | Good | Reject (new design, creator single point) |
| MLS (RFC 9420) | Library exists (openmls) | 1 | **Poor**: commits need DS ordering; out-of-order commits fork (DMLS drafts, FREEK) | Reject for offline |

## 7. Media transfer

- **Evidence:**
  - Briar caps images at 32 KiB (VERIFIED).
  - bitchat's courier excludes media (VERIFIED, ≤ 16 KiB text).
  - iOS BLE throughput reports range from 37 to 400 kbps depending on mode and device (REPORTED, forum data; not our measurement).
- **Decision:** Tier 2 small images only (≤ 16 KiB after on-device re-encoding), sent as k fixed-size chunks plus a manifest (SHA-256 per chunk, existing dependency) with per-chunk receipts, so transfers resume.
- **Erasure coding** (RFC 6330 RaptorQ; Signal SPQR also uses erasure codes for chunking, REPORTED) lets chunks arrive by *different* paths. This is a candidate once the simulator shows a benefit. **Deferred.**

## 8. Versioning and compatibility

- Envelope `version` and HELLO `link_versions` already exist.
- **Rule:** a new inner kind must not change any existing kind's bytes. Recipients that do not know a CONTENT `ctype`:
  - still send RECEIPT_DELIVERED, because decryption succeeded;
  - show an "unsupported message" placeholder;
  - so the sender does not keep spraying.
- **Lesson from Wire:** avoid flag days.
