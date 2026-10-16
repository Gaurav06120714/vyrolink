# Algorithms and Optimization Research — Index

**Created:** 2026-10-15.

**Scope:** the computer-science mechanisms behind the messengers studied in `../messaging-apps/`, evaluated against VYRO LINK's offline, privacy-preserving, store-and-forward design. Each one was then implemented **only where a baseline measurement or a failing test showed a real benefit**.

**Evidence labels:**
- **VERIFIED-IMPL:** source code or spec read.
- **REPORTED-IMPL:** documented by the project, not read.
- **GENERAL:** a published algorithm, not attributed to any messenger.
- **measured:** this repository's benchmarks or simulator.

Simulator results are **not** Bluetooth measurements.

| # | Document | Content |
|---|---|---|
| 01 | [Algorithm catalogue](01-algorithm-catalogue.md) | 45 algorithms: sources, where used, complexity, verdict |
| 02 | [Comparison matrix](02-algorithm-comparison-matrix.md) | Time, space, air bytes, battery, security, difficulty, fit, evidence, decision |
| 03 | [Routing optimization](03-routing-optimization.md) | Baseline findings R1–R6; experiments B1, E1–E4 with numbers; rejected routing families and why |
| 04 | [Sync and conflict resolution (normative)](04-sync-conflict-resolution-spec.md) | Per-state merge rules, HLC (ADR-025), edge cases with tests, limitations |
| 05 | [Database and indexing](05-database-indexing-optimization.md) | Benchmarks; FTS choices; crash recovery; indexes |
| 06 | [Bluetooth performance (design)](06-bluetooth-performance.md) | Verified platform limits; discovery, MTU, batching, flow control, backoff |
| 07 | [Security algorithm review](07-security-algorithm-review.md) | Inventory, attack review of this phase's changes, test coverage |
| 08 | [Architecture optimization](08-architecture-optimization.md) | Patterns kept, adopted, rejected |
| 09 | [Implementation roadmap and results](09-implementation-roadmap.md) | What changed, numbers, what's next |
| 10 | [Attachment transfer design](10-attachment-transfer-design.md) | Chunk + manifest proposal (not implemented) |
| — | [experiments-output.md](experiments-output.md) | Raw simulator output (regenerate with `cargo run -q -p vl-sim --release --bin experiments`) |

## Headline results

1. **244× faster** VACCINE handling at 5,000 stored envelopes, from an indexed lookup that replaces a full scan.
2. **Delta refresh:** −1.7 to −12.7% of link bytes with no delivery or ack change.
3. **Two correctness bugs fixed, each with a test that fails without the fix:**
   - delayed acknowledgements were never shown;
   - timer causality broke under clock skew. Fixed with a **hybrid logical clock**.
4. **Small groups** implemented with no new cryptography: single-writer epoch roster, held-before-invite, removal enforcement.
5. **Biggest remaining cost:** SUMMARY overhead. Compact entries are the next candidate and need a wire change, so they go through an ADR first.
6. **Rejected with reasons:** PRoPHET/MaxProp-style routing (leaks who meets whom), MLS offline, vector clocks and sequence CRDTs (no multi-writer data), minisketch and Merkle reconciliation (round trips, complexity), compression.
