# 07 — Security Algorithm Review

**Date:** 2026-10-15.

**Scope:** every cryptographic or security-relevant algorithm in the core after this phase.
- **No new cryptographic construction was introduced.** Groups reuse pairwise Olm, and the HLC is a non-secret timestamp.

## 1. Primitive and protocol inventory

| Layer | Algorithm | Library | Property relied on | Review notes |
|---|---|---|---|---|
| Pairing | Ed25519-signed cards (vodozemac account) | vodozemac 0.11.1 (`verify_strict`) | Card integrity; in-person exchange gives identity binding | **Hardened:** `verify_card` rejects **all low-order X25519 points** (test `low_order_keys_in_a_signed_card_are_rejected`). vodozemac 0.11.1 also enforces contributory DH (VERIFIED in source) |
| Pairwise keys | X25519 + HKDF-SHA256 (RFC 5869) | x25519-dalek 3, hkdf 0.13 | Shared secret only between the pair; non-contributory results rejected | Unchanged |
| Tags | HMAC-SHA256 truncated to 16 B, per 6 h epoch | hmac 0.13 | Unlinkable to non-participants | Better than bitchat's public-key-derived tags (messaging-apps/apps/bitchat.md) |
| Wrap | ChaCha20-Poly1305, random 96-bit nonce, AAD = header minus mutable fields | chacha20poly1305 0.11 | Confidentiality and integrity against relays | ≤ 2²⁰ envelopes per key direction (ADR-014) |
| Messaging | Olm v1 (Double Ratchet, 3DH) | vodozemac | Forward secrecy and post-compromise security for content; authentication | 8-byte Olm MAC backed by the 16-byte wrap tag; 40 skipped keys max |
| Links | Noise NN (`25519_ChaChaPoly_SHA256`) | snow 0.10 (no `ring`) | Hop confidentiality; unlinkable peers | Unchanged |
| Delivery proof | SHA-256 commitment, preimage = vaccine (ADR-020) | sha2 | Only the recipient (or sender) knows the preimage | Now looked up by index (no semantic change) |
| Storage | SQLCipher (page AES-256 + HMAC) | rusqlite `bundled-sqlcipher` | At-rest confidentiality | **Hardened:** SQLCipher logging off (found via REA); key hex zeroized; the core's conversation-key copy is now `Zeroizing` |
| Wipe | Cryptographic erasure (host deletes the keys) + file removal | — | Unrecoverable after key deletion | Core part tested; key deletion is native (BLOCKED) |

## 2. Attack review for this phase's changes

| Change | Attack considered | Outcome |
|---|---|---|
| HLC (ADR-025) | A hostile contact sends far-future `sent_at_ms` to push our clock forward and break edit/delete windows | Bounded: values > now + 1 h are not absorbed (`hybrid_clock_is_bounded_and_monotonic`) |
| HLC | Clock rollback causes replay acceptance | Replay protection is the seen set + Olm message keys, not timestamps. The HLC only orders LWW registers |
| Late-proof upgrade | Forged "late receipt" turns a message Delivered | No: only `verify_receipt` (wrap-authenticated) or a matching vaccine preimage triggers it. Relay custody still never counts (ADR-015) |
| Delta refresh | A peer exploits the per-session `offered` set to make us withhold mail | The set is per session; a new session re-offers everything. Worst case: one fewer offer in a session that already offered the item |
| Indexed vaccine lookup | Index miss lets a forged vaccine purge | Same predicate (`ack_commitment = H(secret)` and the vaccine flag) as before; conformance-tested on both stores |
| Groups | Non-creator rewrites the roster | Ignored (`NotCreator`) |
| Groups | Replay of an older roster to re-add a removed member | Ignored (`Stale`, epoch must strictly increase) |
| Groups | Non-member posts into the group | Dropped |
| Groups | Removed member posts with an old epoch | Dropped at members that applied the removal |
| Groups | Flood of messages for unknown groups | ≤ 64 held per contact, TTL 7 d + 2 h |
| Groups | Equivocation (different text to different members) | **Not prevented** (pairwise fan-out); documented limitation, same as Signal |
| Groups | Roster reveals members' fingerprints to each other | Inherent to groups; members are shown to members only (inside Olm) |
| Groups | Group copies leak membership to relays | No: each copy is an ordinary pairwise envelope with a pairwise tag. Relays cannot tell group messages from 1:1 messages (the content type is inside Olm) |

## 3. Malicious-input testing status

| Input | Coverage |
|---|---|
| All decoders, including new group content | proptest (4,096 cases) + `decode_all` fuzz, 600 s: **169,698,471 runs, 0 crashes** |
| Link session (vaccine/refresh changes) | `link_session` fuzz, 600 s: **17,821,928 runs, 0 crashes** |
| Corrupted envelopes, forged receipts, misdirected envelopes | vl-crypto tests |
| Resource exhaustion | Orphan cap, group-pending cap, per-tag/per-session/carry quotas, bounded `Deframer`, MAX_ITEMS checked before allocation |
| Storage failure | `Store` errors close the session with `BYE 2` (routing tests) |

## 4. Open security items

R-33 (post-quantum), group transcript consistency, external audit (Phase 9), native key deletion for panic wipe.
