## E1 receipt lifetime (spray L=8)

| scenario | delivery | ack (receipt expires with data) | ack (fresh lifetime, production) | ack p50 min before → after |
|---|---|---|---|---|
| campus (40 nodes, 4 communities, 72 h) | 0.99 | 0.98 | 0.98 | 689 → 691 |
| sparse (30 nodes, uniform, 72 h) | 0.79 | 0.52 | 0.52 | 2075 → 2067 |
| dense event (100 nodes, 12 h) | 0.88 | 0.64 | 0.63 | 324 → 322 |
| campus + 20% black-hole relays | 0.94 | 0.91 | 0.91 | 853 → 854 |
| campus, short windows (10 s mean, 5 s setup) | 0.95 | 0.91 | 0.91 | 1101 → 1101 |
| campus, all nodes always-on (Android FGS backbone) | 1.00 | 1.00 | 1.00 | 411 → 411 |

## E2 link bytes by message type (spray L=8, production receipts)

| scenario | SUMMARY share | VACCINE share | ENVELOPE share | other share | SUMMARY KiB per session |
|---|---|---|---|---|---|
| campus (40 nodes, 4 communities, 72 h) | 0.64 | 0.19 | 0.14 | 0.04 | 3.31 |
| sparse (30 nodes, uniform, 72 h) | 0.30 | 0.04 | 0.55 | 0.10 | 3.35 |
| dense event (100 nodes, 12 h) | 0.53 | 0.06 | 0.33 | 0.07 | 3.06 |
| campus + 20% black-hole relays | 0.62 | 0.18 | 0.16 | 0.05 | 2.41 |
| campus, short windows (10 s mean, 5 s setup) | 0.59 | 0.16 | 0.21 | 0.05 | 1.95 |
| campus, all nodes always-on (Android FGS backbone) | 0.67 | 0.22 | 0.09 | 0.03 | 3.23 |

## E3 why SUMMARY is large (spray L=8)

| scenario | entries per SUMMARY | redundant entries (peer already held or saw) | summaries from mid-session refresh |
|---|---|---|---|
| campus (40 nodes, 4 communities, 72 h) | 43.1 | 0.54 | 0.05 |
| sparse (30 nodes, uniform, 72 h) | 31.2 | 0.34 | 0.32 |
| dense event (100 nodes, 12 h) | 38.7 | 0.13 | 0.10 |
| campus + 20% black-hole relays | 31.2 | 0.53 | 0.04 |
| campus, short windows (10 s mean, 5 s setup) | 40.9 | 0.54 | 0.07 |
| campus, all nodes always-on (Android FGS backbone) | 43.0 | 0.55 | 0.03 |

## E4 delta refresh (spray L=8): full refresh → delta refresh

| scenario | delivery | ack | total link KiB | SUMMARY KiB | VACCINE KiB |
|---|---|---|---|---|---|
| campus (40 nodes, 4 communities, 72 h) | 0.99 → 0.99 | 0.98 → 0.98 | 11533 → 11217 | 7379 → 7129 | 2216 → 2152 |
| sparse (30 nodes, uniform, 72 h) | 0.79 → 0.79 | 0.52 → 0.52 | 2231 → 1947 | 846 → 596 | 113 → 86 |
| dense event (100 nodes, 12 h) | 0.88 → 0.88 | 0.63 → 0.63 | 10279 → 9670 | 5700 → 5146 | 679 → 628 |
| campus + 20% black-hole relays | 0.94 → 0.94 | 0.91 → 0.91 | 8594 → 8357 | 5355 → 5167 | 1530 → 1483 |
| campus, short windows (10 s mean, 5 s setup) | 0.95 → 0.95 | 0.91 → 0.91 | 7466 → 7143 | 4457 → 4194 | 1169 → 1112 |
| campus, all nodes always-on (Android FGS backbone) | 1.00 → 1.00 | 1.00 → 1.00 | 18137 → 17829 | 12137 → 11900 | 3958 → 3888 |
