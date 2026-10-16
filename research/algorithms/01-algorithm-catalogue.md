# 01 — Algorithm Catalogue

**Accessed:** 2026-10-15.

**Evidence labels:**
- **VERIFIED-IMPL:** read in that project's source at a pinned commit (see `../messaging-apps/apps/`) or in its normative specification.
- **REPORTED-IMPL:** the project documents using it, but we did not read the code.
- **GENERAL:** a published algorithm that VYRO LINK *could* adapt; no claim that any messenger uses it.

**Complexity notation:**
- N = envelopes stored, V = vaccines per message, E = summary entries, M = messages in a conversation, G = group members, k = items per batch.

For each algorithm: what it is, its original source, where it is documented, its complexity, pros and cons, security notes, and the **verdict for VYRO LINK**:
- **KEEP:** already in use, still the right choice.
- **ADOPTED:** implemented in this phase, with a measured or tested benefit.
- **CANDIDATE:** promising; needs a design or wire change first.
- **REJECTED:** with the reason.

## A. Cryptographic protocols

| # | Algorithm | Source | Where documented | Complexity / cost | Verdict |
|---|---|---|---|---|---|
| A1 | **Double Ratchet** (symmetric-key ratchet + DH ratchet; per-message keys; forward secrecy and post-compromise security) | Perrin & Marlinspike, *The Double Ratchet Algorithm*, Signal spec rev. 1 (2016) | VERIFIED-IMPL: Signal/WhatsApp (whitepapers); **vodozemac Olm** (our dependency, read at 0.11.1); Matrix | O(1) per message; a DH step when the sender changes; stores ≤ 40 skipped keys (vodozemac `MAX_MESSAGE_KEYS`) | **KEEP** (Olm). The 40-key limit is a known constraint (Doc 20 §5.5) |
| A2 | X3DH / PQXDH (asynchronous authenticated key agreement with prekeys) | Marlinspike & Perrin 2016; Kret & Schmidt 2023 (PQXDH) | VERIFIED: Signal spec; REPORTED: WhatsApp | 3–4 DH (+1 KEM) per session start | **KEEP the Olm 3DH equivalent**: in-person signed cards replace the prekey server. PQ is deferred (R-33) |
| A3 | SPQR / Triple Ratchet (ML-KEM-768 ratchet, erasure-coded chunks) | Signal blog 2025-10-02; Eurocrypt/USENIX Sec 2025 | VERIFIED (Signal blog) | KEM ciphertexts ~1 KB chunked over many messages | **REJECTED for now**: no reviewed Rust implementation fits Olm; a custom integration would be new cryptography. R-33 tracks it |
| A4 | **Noise NN** (ephemeral-ephemeral handshake, then ChaChaPoly transport) | Perrin, *The Noise Protocol Framework* rev 34 | VERIFIED-IMPL: our `snow` usage; bitchat uses Noise XX (VERIFIED) | 2 messages, 2 DH | **KEEP**. NN gives unlinkable hop links. XX would expose static keys to strangers |
| A5 | AEAD with random 96-bit nonces (ChaCha20-Poly1305, RFC 8439) | RFC 8439 | VERIFIED-IMPL: our wrap | O(n). The collision bound at 2²⁰ messages per key is negligible (ADR-014) | **KEEP** |
| A6 | HKDF (extract-then-expand) | RFC 5869 | VERIFIED-IMPL: our pairwise keys | O(1) | **KEEP** |
| A7 | **Hash commitment / preimage proof** (vaccine = secret, commitment = H(secret)) | Folklore; DTN "anti-packets": Zhang, Neglia, Kurose & Towsley, *Performance modeling of epidemic routing*, Computer Networks 2007 | VERIFIED-IMPL: ours (ADR-020) | 1 hash per check | **KEEP** |
| A8 | MLS / TreeKEM (RFC 9420) | IETF 2023 | VERIFIED: RFC; REPORTED-IMPL: Wire, RCS, Discord DAVE | O(log G) per commit, but commits need a delivery service to order them | **REJECTED offline** (ADR-024). Forks under reordering (DMLS drafts) |
| A9 | Sender Keys / Megolm (one symmetric ratchet per sender, distributed pairwise) | Signal; Matrix Megolm spec | VERIFIED-IMPL: vodozemac `GroupSession` exists in our dependency | O(1) encrypt; O(G) key distribution per rotation | **CANDIDATE**. It cuts CPU, not routing cost, unless a group tag is designed. Deferred (ADR-024) |
| A10 | Pairwise fan-out for groups | Early Signal; iMessage (REPORTED) | REPORTED-IMPL | O(G) encryptions and envelopes per message | **ADOPTED** (ADR-024; implemented this phase) |

## B. Ordering, time and conflict resolution

| # | Algorithm | Source | Where documented | Complexity | Verdict |
|---|---|---|---|---|---|
| B1 | Lamport timestamps | Lamport, *Time, Clocks, and the Ordering of Events*, CACM 1978 | GENERAL | 8 bytes per event | Subsumed by B3 |
| B2 | Vector clocks | Fidge 1988; Mattern 1989 | GENERAL; Dynamo-style stores | O(writers) bytes per event | **REJECTED**. Every VYRO LINK register has one writer, or two writers (the timer); vector metadata would add bytes with no decision it could change (04 §2) |
| B3 | **Hybrid Logical Clock** (max(physical, last+1); absorb remote; bounded drift) | Kulkarni, Demirbas, Madappa, Avva & Leone, *Logical Physical Clocks*, OPODIS 2014 | REPORTED-IMPL: CockroachDB, YugabyteDB | O(1), 8 bytes (reuses `sent_at_ms`) | **ADOPTED**. It fixed a real clock-skew bug (03 §5, 04 §3) |
| B4 | Last-writer-wins register | Shapiro, Preguiça, Baquero & Zawirski, *CRDTs*, SSS 2011 | VERIFIED-IMPL: Briar timer (source), Delta Chat reactions (source) | O(1) | **KEEP** for single-writer registers (reactions, edits, timer) |
| B5 | Tombstones (delete markers that outlive the target) | CRDT literature (2P-set) | VERIFIED-IMPL: Delta Chat `insert_tombstone` | O(1) each, bounded by TTL | **KEEP** (ADR-023), with a 7 d + 2 h TTL |
| B6 | Operation-based CRDTs / sequence CRDTs (RGA, Yjs, Automerge) | Roh et al. 2011; Kleppmann et al. | GENERAL (collaborative editors) | O(ops) metadata | **REJECTED**. No multi-writer text: an edit replaces the whole text (≤ 10 per message) |
| B7 | Event sourcing (state = fold of an append-only log) | Fowler 2005 | REPORTED-IMPL: Matrix (event DAG) | O(events) replay | Partially: events are applied idempotently into materialised tables; no replay log (storage cost) |
| B8 | Idempotent receivers / dedup by message ID | Folklore | VERIFIED-IMPL: ours (seen set, `INSERT OR IGNORE`), bitchat seen cache | O(1) per message | **KEEP** |
| B9 | Single-writer epoch counter (total order of group state) | General (leader-based configuration) | VERIFIED-IMPL: bitchat group key epoch (source) | O(1) | **ADOPTED** for groups |

## C. Duplicate suppression and set reconciliation

| # | Algorithm | Source | Where documented | Bits per item / cost | Verdict |
|---|---|---|---|---|---|
| C1 | Exact seen set (hash index with expiry) | Folklore | VERIFIED-IMPL: ours (SQLite table), bitchat | 16 B + index | **KEEP**. A false positive would silently drop real mail |
| C2 | Bloom filter | Bloom, CACM 1970 | GENERAL | ≈ 1.44·log₂(1/p): 29 bits at p = 2⁻²⁰ | Dominated by C3 |
| C3 | Golomb-coded set | Putze, Sanders & Singler 2007; BIP158 | **VERIFIED-IMPL: bitchat `REQUEST_SYNC`** (source); Bitcoin | ≈ 20 bits at 2⁻²⁰ (~70% of a Bloom filter) | **CANDIDATE** for SUMMARY redundancy (03 §4) |
| C4 | Minisketch / IBLT (PinSketch) | Naumenko et al., *Erlay*, CCS 2019 | REPORTED-IMPL: Bitcoin Core | Size ∝ difference; O(d²) decode | **REJECTED**: sets are ≤ 128 items per message; decode cost and complexity not justified |
| C5 | Merkle trees / range-based set reconciliation | Merkle 1987; Meyer, *Range-Based Set Reconciliation* 2023 | REPORTED-IMPL: Negentropy (Nostr) | O(d·log N) round trips | **REJECTED**: multiple round trips are costly in short BLE encounters |
| C6 | **Indexed lookup instead of a full scan** | Folklore | Ours | O(N) → O(log N + k) | **ADOPTED**: `Store::vaccine_holders`, 244× faster (05 §2) |

## D. Delay-tolerant routing

| # | Algorithm | Source | Where documented | Cost | Verdict |
|---|---|---|---|---|---|
| D1 | Epidemic | Vahdat & Becker 2000 | REPORTED-IMPL: FireChat-style flooding; bitchat mesh flood (TTL) | Unbounded copies | REJECTED as default (Doc 22: 3–6× the transfers) |
| D2 | **Binary spray-and-wait** | Spyropoulos, Psounis & Raghavendra, WDTN 2005 | VERIFIED-IMPL: ours; **bitchat courier `maxCopies = 8`** | ≤ L copies | **KEEP** (ADR-021). Binary spraying is optimal for i.i.d. mobility (same paper) |
| D3 | Spray-and-focus (utility-based wait phase) | Spyropoulos et al., PerCom-W 2007 | GENERAL | Destination-specific "last seen" timers | **REJECTED**: needs destination identity, which breaks our pairwise unlinkable tags |
| D4 | PRoPHET (delivery predictability with aging and transitivity) | Lindgren, Doria & Schelén 2003; **RFC 6693** (2012) | REPORTED-IMPL: DTN2 reference | O(destinations) state; exchanged per encounter | **REJECTED**: advertising per-destination predictabilities reveals who meets whom. The privacy model forbids it (03 §3) |
| D5 | MaxProp / RAPID (utility from encounter history; acks flooded) | Burgess et al., INFOCOM 2006; Balasubramanian et al., SIGCOMM 2007 | GENERAL | Per-destination estimates | REJECTED (same privacy reason); MaxProp's *ack flooding* idea is already covered by vaccines (A7) |
| D6 | Social/centrality forwarding (BUBBLE Rap) | Hui, Crowcroft & Yoneki, MobiHoc 2008 | GENERAL | Per-node centrality | **CANDIDATE**: a coarse, destination-free activity bucket in HELLO (03 §6); not yet simulated |
| D7 | **Anti-packets / vaccines** (IMMUNE, VACCINE variants) | Zhang et al. 2007 | VERIFIED-IMPL: ours | 20 B per vaccine | **KEEP**. Restricted to holders for DoS safety |
| D8 | Store-and-forward custody with quotas | DTN Bundle Protocol custody (RFC 9171); bitchat courier quotas | VERIFIED-IMPL: bitchat `CourierStore` (40 envelopes, per-depositor caps) | — | **KEEP** (ours: 32 per session, 8 per tag, 10 MiB) |
| D9 | TTL / expiry + hop limit | RFC 9171 lifetime | VERIFIED-IMPL: ours, bitchat (24 h) | O(1) | **KEEP** |

## E. Transport, flow control and scheduling (for native BLE hosts, Phase 7)

| # | Algorithm | Source | Verdict |
|---|---|---|---|
| E1 | Exponential backoff with full jitter | Brooker, *Exponential Backoff And Jitter*, AWS Architecture Blog 2015 | **CANDIDATE** for reconnect scheduling (06 §4) |
| E2 | Sliding window / selective acknowledgement | RFC 793, RFC 2018 | **REJECTED** above GATT/L2CAP: both give reliable, ordered delivery per connection. Selective ack exists at the custody level (CUSTODY per envelope) |
| E3 | Fragmentation and reassembly with bounded buffers | — | VERIFIED-IMPL: our `Deframer` (one frame buffered, poison on oversize); bitchat fragments (VERIFIED) — **KEEP** |
| E4 | Credit/backpressure by refusal (reject-when-full) | — | **KEEP** (custody `RejectedFull`) |
| E5 | Adaptive batching (send many summary entries and envelopes per write) | — | **KEEP**. Frames already batch per message; native writes should fill the negotiated MTU (06 §3) |
| E6 | Compression | — | **REJECTED** (decompression-bomb history; Doc 06; Bridgefy) |

## F. Storage and indexing

| # | Algorithm | Source | Verdict |
|---|---|---|---|
| F1 | Write-ahead logging (SQLite WAL) + `synchronous=FULL` | sqlite.org/wal.html | **KEEP**: crash-safe; 220 µs per message insert measured (05) |
| F2 | B-tree secondary indexes | SQLite | **KEEP**. Added only where a measured query needs one |
| F3 | FTS5 inverted index (`unicode61`; `trigram` optional) | sqlite.org/fts5.html | **KEEP**: 2.1 ms search at 10 k messages |
| F4 | Transaction batching | — | **CANDIDATE**: per-vaccine fsyncs dominate the remaining 11–16 ms per VACCINE message (05 §2) |
| F5 | Content-addressed chunks + manifest (SHA-256), resumable by missing-chunk set | BitTorrent BEP 3; Briar/XFTP chunking (REPORTED) | **CANDIDATE** for attachments (10) |
