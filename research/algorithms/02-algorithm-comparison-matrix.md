# 02 — Algorithm Comparison Matrix

**Scales:**
- Cost columns: **L** low, **M** medium, **H** high.
- *Fit*: compatibility with VYRO LINK's privacy model (pairwise unlinkable tags, no global identities), BLE and intermittent links, and iOS/Android background limits.

**Evidence levels:**
- **measured:** this repo's benchmarks or simulator (03, 05);
- **analytic:** derived;
- **literature:** cited in 01.

| Problem | Option | Time | Space / metadata | Air bytes | Battery | Security / privacy | Difficulty | Fit | Evidence | Decision |
|---|---|---|---|---|---|---|---|---|---|---|
| Vaccine lookup on receipt of VACCINE | Full scan of stored envelopes (old) | O(V·N) — **3,808.8 ms** at N = 5,000 | — | — | **H** (CPU per encounter) | — | — | — | measured | replaced |
| | **Indexed `vaccine_holders`** | O(V·log N) — **15.6 ms** | Existing index | — | L | Same rules | L | ✓ | measured | **ADOPTED** |
| Concurrent writes to the shared timer | Physical-clock LWW (old) | O(1) | 8 B | — | — | Clock-skew causality bug (test `timer_change_made_after_seeing_a_skewed_change_wins` failed) | — | ✗ | tested | replaced |
| | **HLC + LWW** | O(1) | 8 B (same field) | 0 | L | Remote drift bounded to 1 h | L | ✓ | tested | **ADOPTED** |
| | Vector clocks | O(writers) | 16+ B per event | + | L | — | M | ✓ | analytic | rejected (no benefit with ≤ 2 writers) |
| Mid-session refresh | Full VACCINE + SUMMARY (old) | O(N) | — | 100% | M | — | — | ✓ | measured | replaced |
| | **Delta refresh** | O(N) | O(E) per session | **−2 to −13% total** | L | Same | L | ✓ | measured (E4) | **ADOPTED** |
| SUMMARY redundancy (13–54% of entries already held or seen by the peer) | Compact entries (8-byte truncated id + tag), link v2 | O(E) | — | ≈ −45% SUMMARY, ≈ −17 to −30% total (estimate: share × 0.447) | L | Truncated tags: 2⁻⁶⁴ false match, resolved by the full tag on the envelope | M (wire v2 + negotiation) | ✓ | analytic | **CANDIDATE** (top) |
| | GCS "have" filter before SUMMARY (bitchat-style) | O(S log S) to build | ≈ 12–20 bits per held/seen ID | Net ≈ −14% SUMMARY at S ≈ 300 | M | Leaks the set of envelope IDs seen (IDs are random and already shown in SUMMARY) | M + 1 round trip | ~ | analytic | candidate (second) |
| | Range-based Merkle reconciliation | O(d log N) | — | Small, but several round trips | M | — | H | ✗ (short encounters) | literature | rejected |
| Delivery-proof spread | Vaccines only for held envelopes (current) | O(V log N) | 20 B per vaccine | small | L | Forged vaccines cannot consume storage | — | ✓ | — | **KEEP** |
| | Epidemic vaccine flooding | O(V) | unbounded | M | M | Storage DoS | L | ✗ | analytic | rejected |
| Late acknowledgement | Expired/DeliveryUnknown terminal (old) | — | — | — | — | User never sees proven delivery | — | ✗ | tested | replaced |
| | **Authenticated proof upgrades Expired/DeliveryUnknown** | O(1) | — | 0 | — | Proof remains receipt or vaccine only (ADR-015/020) | L | ✓ | tested | **ADOPTED** |
| Replication | Binary spray L = 8 (current) | O(1) | 1 B | baseline | M | — | — | ✓ | Doc 22 | **KEEP** |
| | PRoPHET / MaxProp / spray-and-focus | O(D) | Per-destination tables | + | M | **Reveals who meets whom** | M | ✗ | literature | rejected |
| | Activity-weighted spray (destination-free) | O(1) | 1 B in HELLO TLV | ≈ | ? | Coarse activity fingerprint | M | ~ | — | candidate (simulate next) |
| Groups | **Pairwise fan-out** | O(G) | 0 | G× | G× | Pairwise authentication; no transcript consistency | L | ✓ | tested | **ADOPTED** |
| | Megolm + group tag | O(1) encrypt | Session state | 1× | L | Needs a reviewed group-addressing design | H | ~ | analytic | deferred |
| | MLS | O(log G) | Tree state | 1× | L | Commit ordering needs a delivery service | H | ✗ | literature | rejected |
| Search | FTS5 `unicode61` (current) | 2.1 ms at 10 k | Index inside SQLCipher | — | L | Encrypted at rest | — | ✓ | measured | **KEEP** |
| | FTS5 `trigram` | similar | ≈ 3× index | — | L | same | L | ✓ | literature | option for CJK |
| Local inserts | WAL + FULL sync (current) | 220 µs per message | — | — | L | Crash-safe | — | ✓ | measured | **KEEP**, no change needed |
