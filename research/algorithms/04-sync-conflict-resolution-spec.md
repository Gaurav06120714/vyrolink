# 04 — Synchronization and Conflict-Resolution Specification

**Status:** normative for the core as of 2026-10-15. Wire bytes are in Doc 21 §4.1–§4.2.

**Principle:** use the **simplest bounded state machine that is order-independent**. Reach for CRDT machinery or causal metadata only when a register has more than one writer, and measure the metadata it would cost.

## 1. Inventory of replicated state

| State | Writers | Data type | Merge rule | Why this, not more |
|---|---|---|---|---|
| Message body (1:1) | Author only | Immutable value + single-writer edit register | Edit with greatest `(seq, sent_at_ms)` wins; only within 24 h; ≤ 10 edits | Single writer: no concurrency. `seq` is a per-message Lamport counter |
| Reactions | Each participant writes only its own slot | Map: author → LWW register | Per (target, author), greatest `(sent_at_ms, event_id)` wins; 0 = removal marker | Single writer per slot. Ties broken by the random event ID, so deterministic |
| Delete-for-everyone | Author only | Tombstone (2P-set "remove" side) | Idempotent; applies inside a 72 h window; outlives an absent target for 7 d + 2 h | Delta Chat pattern (VERIFIED) |
| Delete-for-me | Local | Local tombstone | Suppresses duplicate re-insertion; expires after TTL | Prevents a re-delivered copy from reappearing |
| Disappearing timer | **Both participants** | **Multi-writer LWW register** | Greatest HLC timestamp wins | Two writers, so causality matters. HLC orders "change made after seeing the other's change" correctly without vector clocks (§3) |
| Read state, drafts, pins, stars, blocked | Local | Local flags | — | Never replicated |
| Group roster | **Creator only** | Single-writer register with **epoch** | Strictly greater epoch wins; other writers are ignored | Total order without consensus (bitchat-style epoch, VERIFIED) |
| Group messages | Each member | Append-only set keyed by `group_msg_id` | Union; dedup by ID | Pairwise copies share one logical ID |

**Vector clocks and CRDT sequences were rejected:**
- No register has more than two writers.
- Text is replaced, not merged.
- Vector clocks would add O(writers) bytes to every event and change no decision that HLC + LWW makes for these types.

## 2. Event admission and bounds (anti-abuse)

1. **Authentication first.** An event counts only after the pairwise wrap and the Olm session authenticate its author (contact).
2. **Author checks.**
   - Edits and deletes need `event.author == target.author`.
   - Reactions need the target in the author's conversation.
   - Group text needs the author to be a current member.
3. **Orphans.** Events whose target is unknown are stored keyed by target. Limits: ≤ 512 per contact, expiring after 7 d + 2 h. Group messages held for a missing roster: ≤ 64 per contact, same TTL.
4. **Idempotency.** Every apply is an upsert with a monotone condition, so replays are no-ops.

## 3. Hybrid logical clock (ADR-025)

| Rule | Value |
|---|---|
| Send | `sent_at_ms = max(now_ms, hlc + 1)`; persist `hlc = sent_at_ms` |
| Receive | `hlc = max(hlc, remote_sent_at_ms)` **if** `remote ≤ now_ms + MAX_CLOCK_DRIFT_MS` (1 h) |
| Persistence | `clock` table in the Class C database (survives restarts) |
| Wire | None: `sent_at_ms` is the existing inner-header field, now filled from the HLC |

**Bug fixed** (test `timer_change_made_after_seeing_a_skewed_change_wins`):
- Alice's clock runs 30 min fast and she sets the timer.
- Bob sees it and turns it off.
- *Before:* Bob's own change was rejected even locally, and Alice kept the old value.
- *After:* Bob's change wins on both devices.

**Also tested** (`hybrid_clock_is_bounded_and_monotonic`):
- A contact clock 2 h ahead is **not** absorbed.
- A local clock stepping backwards still yields increasing timestamps, so the newest reaction wins.

**Residual limit:** with skew above 1 h, behaviour degrades to plain physical-clock LWW. That is the previous behaviour, not worse.

## 4. Edge cases (each with its test)

| Case | Behaviour | Test |
|---|---|---|
| Duplicate envelope | Seen set rejects it; `INSERT OR IGNORE` | routing scenarios; `delete_for_me_is_local_and_suppresses_duplicates` |
| Edit before original | Stored keyed by target; applied on arrival | `edits_converge_and_respect_author_and_window` |
| Delete before original | Tombstone; the late original is stored as deleted | `delete_arriving_before_its_target_via_relays` (4 real relays) |
| Delete after the target's envelope expired | Tombstone still applies to the stored message (window measured on the author's clock) | `delete_for_everyone_rules` |
| Concurrent reactions | Per-author slots, so they never conflict; same author: LWW | `reactions_converge_regardless_of_arrival_order` (3 orders × 2) |
| Group message before invite | Held, then accepted when the roster arrives | `message_before_invite_is_held_then_accepted`, `group_pending_is_bounded_and_expires` |
| Group membership change | Higher epoch only, from the creator only; removed members are informed | `stale_roster_replay_is_ignored_and_removal_is_enforced`, `removed_member_cannot_post_to_remaining_members`, `leaving_asks_the_creator_to_remove_me` |
| Delayed acknowledgement | Authenticated proof upgrades `Expired`/`DeliveryUnknown` | `receipt_arriving_after_sender_expiry_still_proves_delivery` |
| Replay of an old roster | `Stale` | same as above |
| Forged roster or post by a non-creator / non-member | Ignored / dropped | `only_the_creator_changes_members_and_only_members_post` |
| Malicious relay modifying an envelope | AEAD failure, dropped (AAD covers the header except mutable hop/budget fields) | vl-crypto `forged_receipts_and_misdirected_envelopes_fail` |
| Unknown future content type | Acknowledged, not shown | `unknown_content_type_is_acknowledged_and_ignored` |

## 5. Known limitations (honest)

- **Group transcript consistency:** with pairwise fan-out a member can send different texts to different members (equivocation). Signal's groups have the same property. Detecting it would need hashing over the transcript, which is a new design and deferred.
- **Removal race:** a group message sent *before* a removal but arriving *after* it is dropped by recipients that already applied the removal.
- **Group reactions, edits and deletes:** not implemented (the 1:1 event tables are keyed by contact conversation). Deferred.
