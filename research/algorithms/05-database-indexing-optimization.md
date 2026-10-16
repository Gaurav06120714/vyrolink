# 05 — Database and Indexing Optimization Report

**Date:** 2026-10-15. **Engine:** SQLCipher (bundled via rusqlite 0.40.2 / libsqlite3-sys 0.38.2): WAL, `synchronous=FULL`, `secure_delete=ON`, page-level AES via CommonCrypto (Apple).

**Hardware:** Apple M2 Pro, release builds, real encrypted files, median of 5 runs. Phone numbers will differ; these are **relative baselines**, not device claims.

## 1. Conversation database (Class C): `bench_conversation`, 10,000 messages, 10 contacts

| Operation | Median | Notes |
|---|---|---|
| `insert_message` incl. FTS5 index + WAL fsync | **220 µs / message** | One transaction per message (crash-safe) |
| Search, 2 prefix terms, top 50 | **2.14 ms** | FTS5 `unicode61`, quoted prefix terms (injection-safe) |
| Conversation page (50 newest) | **0.12 ms** | Index `messages_conversation(contact_id, created_at)` |
| Message by ID | **0.028 ms** | Primary key |
| `upsert_reaction` | **0.05 ms / event** | Includes the admissibility check |

**Decision: no change.** These costs are far below interaction thresholds, and optimising them would be complexity without benefit.

## 2. Relay database (Class R): `bench_relay`

| Stored envelopes | VACCINE before | after | HELLO→SUMMARY | REQUEST(32) |
|---|---|---|---|---|
| 500 | 348.5 ms | 11.4 ms | 1.9 ms | 0.2 ms |
| 2,000 | 1,441.2 ms | 11.7 ms | 11.8 ms | 1.1 ms |
| 5,000 | 3,808.8 ms | 15.6 ms | 31.2 ms | 1.2 ms |

- **Adopted:** the indexed `vaccine_holders` (03 §2).
- **Remaining candidates:**
  - **Transaction batching** of the up to 64 `add_vaccine` commits per VACCINE message. Each fsync is about 0.15–0.2 ms here; on phone flash it is likely more (unmeasured). It needs a batch `Store` method; the engine's per-call atomicity contract would be kept.
  - **SUMMARY construction** loads every unexpired envelope (31 ms at 5,000). An `ORDER BY` with `LIMIT 128` on indexed columns would avoid decoding envelopes never offered. Benefit grows with store size; implement if the carry cap is raised.

## 3. Full-text search choices

| Option | Behaviour | Cost | Decision |
|---|---|---|---|
| FTS5 `unicode61` (current) | Word tokens, case- and diacritic-folding, prefix queries | Smallest index | **KEEP** |
| FTS5 `trigram` (SQLite ≥ 3.34) | Substring search; works for CJK; queries < 3 chars cannot match | ≈ 3× index (literature) | Offer per locale later |
| External engine (tantivy, etc.) | Rich ranking | A second, unencrypted-by-default index; new dependency | **Rejected** |

**Security:**
- The FTS shadow tables live inside the SQLCipher file. This is tested: `plaintext_never_reaches_disk` checks that the search index and drafts are not readable on disk.
- Deletion, edits and disappearing messages re-index or remove entries. This is tested (`delete_for_everyone_rules`, `disappearing_timer_is_last_writer_wins_and_purges`).

## 4. Crash recovery and garbage collection

- **Crash recovery:** WAL + `synchronous=FULL` + one transaction per logical change (message insert, state transition, tombstone, roster) means a crash leaves either the old or the new state.
- **Routing store:** each `Store` call is atomic (contract in `vl-routing`).
- **Garbage collection** (`maintain()` → `purge_expired`): expired envelopes, seen IDs, vaccines, disappearing messages, orphan events, held group messages, local tombstones. All are bounded by TTL.

## 5. Indexes added this phase

| Index | Query served |
|---|---|
| `messages_expire(expire_at)` partial | Disappearing-message purge |
| `group_messages_order(group_id, created_at)` | Group conversation page |
| `group_pending_group(group_id)` | Release held messages |
| `group_copies_message`, `group_copies_envelope` | Delivery counts; vaccine/receipt mapping |
