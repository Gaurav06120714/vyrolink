# 08 — Architecture Optimization Report

**Question:** which architectural patterns from the messaging ecosystem make VYRO LINK more reliable or efficient, and which add cost without benefit?

| Pattern | Seen in (label) | VYRO LINK status | Evidence / reason |
|---|---|---|---|
| Shared Rust core + thin native hosts + generated bindings | Signal libsignal, Element X matrix-rust-sdk, Wire core-crypto, Threema libthreema, Delta Chat core (REPORTED/VERIFIED) | **KEEP** (ADR-013/022) | No business logic is duplicated: groups, HLC and delta refresh were added once, in Rust; Swift calls them through UniFFI (22/22 interop checks) |
| Pull-based byte API (host feeds bytes, core returns bytes) | Ours (ADR-022) | **KEEP** | Lets the host own BLE timing and flow control (06 §3) |
| Event-keyed materialised state + idempotent upserts (event sourcing without a replay log) | Matrix events (REPORTED), Delta Chat (VERIFIED) | **KEEP** (ADR-023) | Out-of-order safety at O(1) per event; a full replay log would cost storage on phones |
| Single-writer registers + epochs instead of consensus | bitchat groups (VERIFIED) | **ADOPTED** for groups | No coordinator exists offline |
| Hybrid logical clocks | CockroachDB (REPORTED) | **ADOPTED** (ADR-025) | Fixes causality under skew at zero wire cost |
| Bounded queues + reject-when-full (backpressure) | bitchat courier quotas (VERIFIED) | **KEEP** | Custody never silently dropped |
| Indexed lookups on hot paths | — | **ADOPTED** | 244× on VACCINE handling |
| Delta synchronisation within a session | bitchat `sinceTimestamp` cursor (VERIFIED) | **ADOPTED** (delta refresh) | −2 to −13% link bytes, no loss |
| Server mailbox / multi-device sync | Messenger Labyrinth, Session config messages (REPORTED) | **REJECTED / DEFERRED** | No servers; Session gave up forward secrecy for it |
| Microservice-style job queues, caches | Server messengers | **REJECTED** | The phone is a single process; SQLite is fast enough at measured scale (05 §1) |
| Compression | — | **REJECTED** | Decompression-bomb risk; padding to buckets dominates anyway |
| Crash recovery: WAL + atomic store calls | SQLite (VERIFIED docs) | **KEEP** | Each logical change is one transaction |

**Conclusion:**
- **No architectural rewrite is justified.** The measured gains came from three local fixes: an index, a delta, and a clock rule.
- **Two correctness fixes came from careful state-machine and clock review:** late proof, and timer causality.
- The biggest remaining measured cost is SUMMARY bytes. Its fix (compact entries) is a protocol change, staged in 09 with an ADR.
