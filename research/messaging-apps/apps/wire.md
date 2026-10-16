# Wire — research report

**Accessed:** 2026-10-15. **Confidence:** Medium.

| Item | Finding | Label |
|---|---|---|
| Protocol | Migrated from Proteus (per-device, Signal-like) to **MLS**. Waves on 2025-03-05 and 2025-03-30 cut off old app versions and iOS < 16 devices; general availability reported in May 2025 | REPORTED (Wire support/blog; Reddit) |
| Rollout problems | Heavy back-end load during migration; client bugs | REPORTED (Wire blog) |
| Core library | `core-crypto` (Rust) | REPORTED |

**Lesson:** protocol migrations on *deployed* fleets force cut-offs. VYRO LINK must keep **version negotiation** (HELLO `link_versions`, envelope `version`) and avoid flag-day changes. New inner kinds should be **ignorable** by old clients (see 05 §8).

## Sources (accessed 2026-10-15)

- https://support.wire.com/hc/en-us/articles/18666343787293-Migration-from-Proteus-to-MLS-protocol (high)
- https://wire.com/en/blog/mls-is-coming-to-wire-app-learn-more (medium–high)
