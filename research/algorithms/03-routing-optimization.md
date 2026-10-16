# 03 — Routing Optimization Report

**Date:** 2026-10-15.

**Method:**
- Every number below comes from the **real `vl-routing` engine** (exchanging wire-encoded link messages), run by the `vl-sim` discrete-event simulator, or from `vl-store` benchmarks on real SQLCipher files.
- Each simulator cell is the mean of seeds 1–5 on identical encounter traces.
- **These are simulations and local benchmarks, not Bluetooth measurements.**

**Reproduce:**
```bash
cd core && cargo run -q -p vl-sim --release --bin experiments
cargo run -q --release -p vl-store --example bench_relay
```

The full output is in [`experiments-output.md`](experiments-output.md).

## 1. Baseline problems found

| # | Finding | How found |
|---|---|---|
| R1 | `on_vaccine` called `store.all()` once **per vaccine**, decoding every stored envelope from SQLCipher: O(V·N) | Code audit, then benchmark B1 |
| R2 | SUMMARY messages are **30–67% of all link bytes**; envelopes, the actual payload, are only 9–55% | Simulator byte accounting (E2) |
| R3 | 13–54% of SUMMARY entries describe envelopes the peer **already holds or has seen** | E3 |
| R4 | Mid-session refreshes re-sent the full VACCINE and SUMMARY lists (3–32% of summaries are refreshes) | E3 |
| R5 | A receipt arriving after the sender's copy expired could never mark the message Delivered | Audit, then the failing test `receipt_arriving_after_sender_expiry_still_proves_delivery` |
| R6 | The simulator gave receipts the data envelope's expiry; production gives them a fresh lifetime | Audit |

## 2. B1 — indexed vaccine lookup (ADOPTED)

`Store::vaccine_holders(commitment)` is answered by the existing `envelopes(ack_commitment)` index (`RelayDb`), replacing the full scan.

| Stored envelopes | VACCINE (128 items, 64 match) before | after | Speed-up |
|---|---|---|---|
| 500 | 348.5 ms | 11.4 ms | 31× |
| 2,000 | 1,441.2 ms | 11.7 ms | 123× |
| 5,000 | 3,808.8 ms | 15.6 ms | **244×** |

- **Benchmark:** `vl-store/examples/bench_relay.rs`, median of 5, Apple M2 Pro, release build.
- **Correctness:** the shared `Store` conformance test (`vaccine_holders` assertions for `MemStore` and `RelayDb`) and every routing scenario test pass unchanged.
- **Remaining cost (11–16 ms):** about 64 separate `add_vaccine` transactions, each fsynced under `synchronous=FULL`. Batching them is a further candidate (05 §2).

## 3. E1 — receipt lifetime (hypothesis refuted; fidelity fix kept)

The hypothesis was that the weak sparse-network ack ratio comes from receipts expiring with the data. **It does not:**
- ack ratio is unchanged (0.52 → 0.52 sparse; at most 0.01 elsewhere);
- p50 ack latency is unchanged (±8 minutes).

The simulator now models production behaviour (fresh receipt lifetime), but it is reported here as **no measured benefit**.

The separate **production** fix R5 (authenticated proof upgrades `Expired`/`DeliveryUnknown` to Delivered) is a correctness fix for delayed acknowledgements. It is covered by an end-to-end test that fails without the change.

## 4. E2/E3 — where link bytes go

With delta refresh on (final code):

| Scenario | SUMMARY | VACCINE | ENVELOPE | Other | Redundant SUMMARY entries |
|---|---|---|---|---|---|
| Campus | 0.64 | 0.19 | 0.14 | 0.04 | 0.54 |
| Sparse | 0.30 | 0.04 | 0.55 | 0.10 | 0.34 |
| Dense event | 0.53 | 0.06 | 0.33 | 0.07 | 0.13 |
| Campus + 20% black holes | 0.62 | 0.18 | 0.16 | 0.05 | 0.53 |
| Campus, short windows | 0.59 | 0.16 | 0.21 | 0.05 | — |
| Campus, always-on backbone | 0.67 | 0.22 | 0.09 | 0.03 | — |

**Interpretation:** in community mobility the same nodes meet repeatedly, so most of each SUMMARY repeats what the peer already has. Every SUMMARY entry costs 38 bytes.

## 5. E4 — delta refresh (ADOPTED, default on)

Total link bytes fall in every scenario: campus −2.7%, **sparse −12.7%**, dense −5.9%, black-hole −2.8%, short windows −4.3%, backbone −1.7%. Delivery and ack ratios are **identical** in all six.
- **Policy flag:** `delta_refresh` (default `true`).
- **No wire change;** HELLO behaviour is unchanged.

## 6. Candidates evaluated but not implemented

| Candidate | Analysis | Status |
|---|---|---|
| **Compact SUMMARY entries** (link v2: 8-byte envelope-ID prefix, 8-byte tag prefix, expiry, packed bucket+spray = 21 B instead of 38 B) | Saves ≈ 44.7% of SUMMARY bytes, ≈ 17–30% of total link bytes (share × 0.447, from E2). Tag-prefix false match is 2⁻⁶⁴ per tag, resolved on the full envelope tag. Needs link-version negotiation (HELLO already carries versions) and prefix lookups in the stores | **Top candidate** for the next routing change; wire change, so it gets an ADR and vectors first |
| GCS "have" filter before SUMMARY (bitchat `REQUEST_SYNC`, VERIFIED in source) | Saves the redundant 13–54% of entries but costs about 12–20 bits per held/seen ID in the receiver's set, plus one round trip. Net ≈ −14% of SUMMARY at about 300 seen IDs | Second candidate; combine with compact entries only if measurement shows a further gain |
| PRoPHET / MaxProp / RAPID / spray-and-focus | Need per-destination delivery predictability. With pairwise rotating tags, a relay cannot know destinations without nodes advertising which tags they own, which links identities and encounter history | **Rejected** (privacy model) |
| Activity-weighted spray (give more copies to peers that meet more nodes; a destination-free 1-byte bucket in a HELLO TLV, which old receivers ignore) | Literature (BUBBLE Rap) suggests gains under community mobility; it leaks a coarse activity level | Candidate; simulate before any wire change |
| Vaccine flooding beyond holders | Faster proof spread, but unbounded vaccine storage from forged secrets is a DoS vector | Rejected |
| Vaccine acceptance for *seen* (not held) envelopes | In spray-and-wait, relays keep their copy until vaccinated or expired, so holders already cover nearly all nodes that ever carried a copy. Analysis predicts no gain, so it was not implemented | Rejected (analysis) |

## 7. Open problem: sparse-network ack ratio (0.52 vs delivery 0.79)

E1 shows lifetime is not the cause. The remaining hypothesis is **encounter scarcity on the return path**: the receipt must make its own spray-and-wait journey back. Candidates for the next simulator round:
- (a) give receipts their own initial copy budget;
- (b) let the recipient attach its vaccine to every HELLO for longer.

Both are measurable without hardware.
