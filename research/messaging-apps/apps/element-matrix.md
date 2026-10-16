# Element / Matrix — research report

**Accessed:** 2026-10-15. **Confidence:** Medium–high.

| Item | Finding | Label |
|---|---|---|
| Crypto library | **vodozemac**, the Rust Olm/Megolm reference, which **VYRO LINK already uses** (0.11.1). Least Authority audit report dated 2022-03-30 (10 issues) | VERIFIED (Least Authority PDF; matrix.org 2022-05-16) |
| 2026 dispute | A February 2026 researcher post alleged weaknesses; Matrix acknowledged a **missing validation check in Olm key exchange** but called it not practically exploitable | REPORTED (CyberInsider) — **action item for VYRO LINK** |
| Groups | Megolm (sender-key-like ratchet per sender); MLS integration (MSC4256) experimental; DMLS draft | REPORTED |
| Sync | Simplified Sliding Sync (MSC4186) native in Synapse; proxy retired January 2025 | REPORTED |
| Features | Threads, reactions (annotations), edits (replacement events), redactions, spaces, cross-signing verification | REPORTED (Matrix spec; not re-fetched) |

**Lessons:**
- Matrix *redactions* and *replacement events* are an event-referencing model: every change is a new event pointing at the original. That is the right model for an append-only, out-of-order network.
- **Checked 2026-10-15 (VERIFIED in source):** vodozemac **0.11.1**, the version VYRO LINK pins, already enforces `was_contributory()`:
  - `types/curve25519.rs:55` returns `None` on a non-contributory result;
  - that propagates through the 3DH in `olm/shared_secret.rs` and the ratchet steps (`SessionCreationError::NonContributoryKey`, `EncryptionError::NonContributoryKey`);
  - Ed25519 uses `verify_strict`.

  **VYRO LINK also had its own gap:** `verify_card` rejected only the all-zero key, not other low-order points such as u = 1. **Fixed** by generalising it to `is_low_order()` in `vl-crypto/src/identity.rs`, with regression test `low_order_keys_in_a_signed_card_are_rejected`. Keep tracking vodozemac advisories (risk R-NEW-2 in `03-security-privacy-analysis.md`).

## Sources (accessed 2026-10-15)

- https://leastauthority.com/static/publications/LeastAuthority-Matrix_vodozemac_Final_Audit_Report.pdf (high)
- https://matrix.org/blog/2022/05/16/independent-public-audit-of-vodozemac-a-native-rust-reference-implementation-of-matrix-end-to-end-encryption/ (high)
- https://cyberinsider.com/matrix-disputes-confidentiality-break-claims-in-crypto-library/ (medium)
- https://matrix.org/blog/2024/11/14/moving-to-native-sliding-sync/ (high)
- https://datatracker.ietf.org/doc/draft-xue-mls-decentralized/ ; draft-kohbrok-mls-dmls-03 (high, drafts)
