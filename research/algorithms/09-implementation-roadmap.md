# 09 — Algorithm Implementation Roadmap and Results

**Rule applied to every item:** baseline → small isolated change → identical before/after measurement or a failing-then-passing test → keep only if it improves without loss.

## Done this phase (2026-10-15)

| # | Change | Component | Baseline → result | Evidence | Kept? |
|---|---|---|---|---|---|
| 1 | Indexed `Store::vaccine_holders` | vl-routing, vl-store | VACCINE at 5,000 envelopes: 3,808.8 → 15.6 ms (244×) | `bench_relay`; conformance test | ✓ |
| 2 | Late authenticated proof upgrades `Expired`/`DeliveryUnknown` | vl-store state machine | Delivered never shown → shown | `receipt_arriving_after_sender_expiry_still_proves_delivery` (fails without the fix) | ✓ |
| 3 | Hybrid logical clock (ADR-025) | vl-core, vl-crypto `SendParams`, vl-store `clock` | The later timer change lost under 30 min skew → wins | `timer_change_made_after_seeing_a_skewed_change_wins`, `hybrid_clock_is_bounded_and_monotonic` | ✓ |
| 4 | Delta refresh | vl-routing | Link bytes −1.7% … −12.7%, delivery/ack unchanged | Simulator E4 | ✓ (default on) |
| 5 | Simulator receipt lifetime matches production | vl-sim | ack unchanged (hypothesis refuted) | E1 | ✓ (fidelity only) |
| 6 | Simulator byte accounting + redundancy metrics | vl-sim | Found SUMMARY = 30–67% of bytes | E2/E3 | ✓ (instrumentation) |
| 7 | Small groups, pairwise fan-out (ADR-024) | vl-proto, vl-store, vl-core, vl-ffi | New feature | 7 group e2e tests, store pending test, vectors, Swift checks | ✓ |
| — | Conversation-DB optimisation | vl-store | 220 µs insert, 2.1 ms search: already fast | `bench_conversation` | **No change** (not needed) |

## Next, in priority order (hardware-independent first)

| # | Item | Why | Gate before merging |
|---|---|---|---|
| N1 | **Compact SUMMARY entries (link v2)** | Estimated −17 to −30% of all link bytes (03 §6) | ADR + Doc 21 + vectors + version negotiation tests + simulator before/after |
| N2 | Receipt-return experiments (receipt budget; longer vaccine advertising) | Sparse ack ratio 0.52 | Simulator before/after; no regression in delivery |
| N3 | Batch `add_vaccine` transaction | Removes up to 64 fsyncs per VACCINE | `bench_relay` before/after |
| N4 | Activity-weighted spray (HELLO TLV) | Possible delivery gain under community mobility | Simulator first; privacy review of the activity bucket |
| N5 | Attachments (10) | F-14 | ADR for the content types; tests listed in 10 §4 |
| N6 | Shared reconnect scheduler (backoff + jitter + cooldown) in Rust | Same policy on both platforms | Only once native hosts exist |
| N7 | Native panic-wipe key deletion (Keychain/Keystore) | AC-16.5 | Needs native host projects; device test BLOCKED |
| N8 | BLE probes / Experiments A–C | Replace simulator assumptions | **BLOCKED**: Android SDK licences not accepted; no Android phone; iPhone offline |
