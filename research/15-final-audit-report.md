# Document 15 — Final Research Audit Report

**Date:** 2026-10-14 · **Scope:** the whole `VyroLink/research/` package (Docs 00–18) · **Result:** the package is internally consistent after the corrections below. **No experiment has been performed.**

## 1. Method and what was actually checked

| Check | Method | Result |
|---|---|---|
| Documents exist | Directory listing | 19 files: 00–18 |
| Internal Markdown links resolve | Script over all `[..](*.md)` links | All resolve |
| Cross-document IDs resolve | Script: every S/E/H/ADR/R/T identifier used is defined | All resolve (S01–S85, E1–E17, H1–H8, ADR-001–019, R-01–R-32, T1–T13) |
| Source URLs reachable | HTTP request to all 68 register URLs (before section K was added) | 65 × 200; 3 × 403 bot-blocks (S49, S51, S58), content previously retrieved |
| Key parameters consistent | grep across docs (L, cap, expiry, epoch, hop limit, wake budget) | Consistent |
| Disputed platform claims | Re-verified against primary sources (Doc 14 §3) plus Android 16/17 behaviour pages | Upheld; evidence levels upgraded where verified |
| Dependency facts | GitHub API (licence, activity, releases, advisories), crate manifests, audit reports | Recorded in Doc 18 §12 / Doc 12 §K |
| Protocol ↔ security | Manual comparison of Doc 07 vs Doc 06 | **Two defects found and fixed** (C-02, C-03) |
| Routing ↔ delivery states | Manual comparison of Doc 05 §5 / Doc 07 §7 / Doc 14 §7.2 | **Vocabulary conflict and an unauthenticated state fixed** (C-02, C-04) |
| Experiments ↔ risk register | Traceability table §4 | Every risk has a verification route or is accepted |

**Limitation:**
- Docs 02, 03, 09 and 13 were checked for identifiers, parameters and their key claims. They were **not** re-verified sentence by sentence against every source in this phase.
- Full texts of S02, S28 and S29 remain unread (abstracts and summaries only), as already stated in Doc 12.

## 2. Corrections made in this phase

| ID | Finding | Severity | Correction | Files |
|---|---|---|---|---|
| C-01 | ADR-004 (core language) was still provisional | Medium | Resolved by **ADR-013**: Flutter UI + native hosts + Rust core (UniFFI); background never needs Flutter, so R-22 closes | 05, 10, 13, 18 |
| C-02 | **Security defect:** `DELIVERED_DIRECT` let a relay's unauthenticated `DELIVERED_LOCAL` claim show "Delivered" (tags are visible to any relay, and links are anonymous) | **High** (misleading indicator, R-19) | **ADR-015:** only a verified `RECEIPT_DELIVERED` sets the Delivered UI state; `DELIVERED_LOCAL` is diagnostic only | 05, 07, 14, 17, 18 |
| C-03 | Wrap AEAD XChaCha20-Poly1305 has no authoritative specification (stated by its own crate) | Medium | **ADR-014:** ChaCha20-Poly1305 (RFC 8439) with random 96-bit nonces, per-direction keys, 2²⁰ cap, then rotation | 01, 05, 06, 07, 14, 18 |
| C-04 | Delivery-state vocabulary differed across Docs 05/07/08/14 and the brief | Medium | Final model in Doc 18 §7.4 (QUEUED, SENT_TO_NEARBY, ACCEPTED_BY_RELAY, ACKNOWLEDGED, READ, EXPIRED, DELIVERY_UNKNOWN, FAILED; relay-side FORWARDED; recipient-side DELIVERED) with aliases | 05, 07, 08, 14, 18 |
| C-05 | No feedback path when the outer unwrap succeeds but Olm decryption fails | Medium | New inner type `0x22 RECEIPT_UNDECRYPTABLE` | 07, 18 |
| C-06 | No protocol hook for the "iPhone endpoint-only" fallback (Doc 14 §12) | Low | **ADR-019:** `HELLO.relay_class` | 07, 18 |
| C-07 | Dependency evidence incomplete (advisories, audits, MSRV, staleness) | Medium | Added; `qr_flutter` rejected as stale (last push 2024-06-28); Rust MSRV 1.96 noted | 12, 18 |
| C-08 | Find My F5 evidence vague; no final principle list | Low | F5 updated from the PoPETs page (location correlation, 7 days of history); §6 adopt/reject/investigate table added | 02 |
| C-09 | Doc 11 roadmap and parts of Doc 08 overlapped newer plans | Low | Marked superseded (Doc 18 §19; Doc 14) | 08, 11 |
| C-10 | Doc 13 environment spec predated ADR-013 | Low | Note: Rust required; min OS proposals | 13 |
| Earlier X1–X3 | ADR-004 timing; circular E1 threshold; evidence levels | — | Fixed in the previous phase (Doc 14 §2.4) | 04, 08, 11 |

## 3. Consistency verdicts

| Pair | Verdict |
|---|---|
| Protocol (Doc 07 + Doc 18 §7) ↔ security (Doc 06 + Doc 18 §8) | Consistent after C-02, C-03, C-05. The header's AAD coverage, mutable fields, receipts and dedup match the threat model T5/T6/T2 |
| Routing (Doc 18 §7.3) ↔ delivery states (Doc 18 §7.4) | Consistent: carriers keep copies until vaccine/expiry; sender states derive only from custody frames and verified receipts |
| Architecture ↔ platform restrictions | Consistent: background is native + Rust; iPhones initiate to Android; force-quit/pre-unlock non-participation is reflected in the UX (Doc 16 screens 12/13) |
| Roadmap ↔ unresolved experiments | Consistent: Phases 6–10 are gated on GA/GB/GS; only core and prototype work is authorised |
| UI ↔ protocol | "Delivered" only for ACKNOWLEDGED (Doc 16 §7, T-UI-05) |

## 4. Risk → verification traceability

| Risk | Verified by | Status |
|---|---|---|
| R-01, R-02 | Doc 14 A2, A3, A-L | Pending |
| R-03 | E6 | Pending |
| R-04, R-05 | E9 (simulator, Phase 5), E12 (pilot) | Pending |
| R-06 | Doc 14 C + survey | Pending |
| R-07 | Play internal-track submission (Phase 2/7); not an experiment | Pending |
| R-08 | Doc 14 A/C on an aggressive OEM | Pending |
| R-09, R-10 | T-RTE-10, T-SEC-03, E11, B6 | Pending |
| R-11 | E10, T-SEC-01 | Pending |
| R-12 | T-CRY-03…05, E16 | Pending |
| R-13 | T-PRO-05 cross-platform vectors | Pending |
| R-14, R-32 | Licence review (Phase 1/10) | Pending |
| R-15 | Dependency review (CI) | Ongoing |
| R-16 | Phase 1 retrospective | Pending |
| R-17, R-27 | Per-OS-release re-runs (Doc 17 §4) | Ongoing |
| R-18, R-19 | B1–B6, T-RTE-08, T-UI-05, E12 survey | Pending |
| R-20 | Accepted (disclosed) | — |
| R-21 | E17 (deferred feature) | Deferred |
| R-22 | Closed by design (ADR-013) | Closed |
| R-23 | Legal review | Pending |
| R-24 | E15, T-RTE-07 | Pending |
| R-25, R-28 | Obtain devices (U-3) | Open |
| R-26 | Implementation rule (Doc 18 §12) | Addressed |
| R-29 | Deviation log | Ongoing |
| R-30 | Phase 1 UniFFI spike | Pending |
| R-31 | P9 external review scope | Pending |

## 5. Remaining uncertainties (not resolvable from documentation)

1. Background/locked iPhone participation and the per-wake session budget (Doc 14 A).
2. Real multi-hop delivery, persistence and receipt return on devices (B).
3. Battery cost and user acceptance (C).
4. Delivery ratio at realistic densities (E9/E12).
5. Play policy acceptance of the relay FGS.
6. UniFFI async foreign-trait maturity.
7. iOS 27 Bluetooth behaviour changes (no primary notes found).
8. iPhone↔Android Wi-Fi Aware interoperability (deferred feature).
9. Whether Android `connectedDevice` FGS may start from BOOT_COMPLETED on all target versions (inferred, unverified).

## 6. Authoritative documents by topic

| Topic | Authoritative | Supporting / historical |
|---|---|---|
| Final architecture, decisions, APIs, data model, dependencies, roadmap, handoff | **18** | 05 (components, ADR-001…012 rationale), 11 (superseded roadmap) |
| Wire protocol details | **07** (as amended) + **18 §7** | — |
| Security design | **06** (as amended) + **18 §8** | — |
| Delivery states | **18 §7.4** | 07 §7 (historical) |
| UI/UX | **16** | 09 (product rationale) |
| Testing & acceptance | **17** | 08 (E-series definitions) |
| Feasibility experiments & gates | **14** | 08 |
| Platform facts | **04** + **14 §3** | — |
| Find My | **02** | — |
| Prior art & licences | **03** + **18 §12** | — |
| Risks | **10** | — |
| Sources | **12** | — |
| Tools & lab | **13** (+ 18 §12 for development tools) | — |

## 7. Decisions requiring user input

See Doc 18 §20 (U-1…U-10). The blocking ones are:
- **U-1** licence;
- **U-2** approve installs;
- **U-3** Android device(s);
- **U-6** gate rules.
