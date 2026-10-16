# VYRO LINK — Research and System Design Package (v2.0, 2026-10-14)

> **Update 2026-10-15 (later):** the iOS and Android apps are built and run on the simulator and emulator; see [FEATURE-CHECKLIST.md](FEATURE-CHECKLIST.md). All research now lives in this folder.
>
> **Update 2026-10-15:** development has started. The shared Rust core is implemented and tested (Docs 19–22), and the messaging-ecosystem research with its Tier 1 conversation features is done (`messaging-apps/`). Setup lives in [`../SETUP.md`](../SETUP.md); the live status is in [Doc 20](20-development-progress.md). The note below describes Docs 01–18 as written.

**Research only (Docs 01–18).** No application was built, no project was initialised, and nothing was installed. An earlier install attempt was stopped and fully rolled back at the user's request before this research started.

**Status: CONDITIONAL GO. Architecture frozen for the MVP.** Start with **[18-development-blueprint.md](18-development-blueprint.md)** (authoritative) and **[15-final-audit-report.md](15-final-audit-report.md)**.

- **Authorised:** a limited prototype + core development (Phases 1, 2a, 3–5).
- **Not authorised:** the full application until feasibility gates GA/GB/GS pass with real device data.
- **All experiments: NOT TESTED.**

| # | Document | What it answers |
|---|---|---|
| ★ | **[FEATURE-CHECKLIST.md](FEATURE-CHECKLIST.md)** | **Every researched feature and requirement ticked ✅ / 🟡 / ❌ against the built iOS and Android apps (2026-10-15)** |
| — | [RESEARCH.md](RESEARCH.md) | Claude Code skills (now in `~/.claude/skills`) and reference repos |
| 01 | [Executive Research Report](01-executive-report.md) | Findings, feasibility, the 20 final questions, go/no-go |
| 02 | [Apple Find My Technical Research](02-apple-find-my.md) | How Find My works (official vs reverse-engineered), what to learn, what is off-limits |
| 03 | [Existing Systems and Open-Source Analysis](03-existing-systems.md) | bitchat, Briar, Bridgefy, Meshtastic, Serval, BT Mesh, Nearby, DTN; repo inventory; licences |
| 04 | [Wireless Technology and Platform Feasibility](04-wireless-feasibility.md) | Transport matrix, cross-platform discovery matrix, iOS/Android behaviour in 10 states |
| 05 | [System Architecture Specification](05-system-architecture.md) | Architectures A–D, 23 components, 12 diagrams, routing design, ADR-001…012 |
| 06 | [Security and Privacy Specification](06-security-privacy.md) | Threat model, pairing, tags, E2E layering, storage classes, abuse controls |
| 07 | [Protocol and Data Model Specification](07-protocol-data-model.md) | L0–L3 protocol, 62-byte envelope header, message types, data model, delivery states, versioning |
| 08 | [Feasibility and Experimental Validation Plan](08-validation-plan.md) | Hypotheses H1–H8, experiments E1–E17, gates |
| 09 | [MVP Scope and Product Strategy](09-mvp-scope-product-strategy.md) | Users, feature classification, competitive analysis, differentiation |
| 10 | [Technical Risk Register](10-risk-register.md) | R-01…R-32, decision gates G0–G5 |
| 11 | [Future Development Roadmap](11-roadmap.md) | Phases P0–P10 with effort ranges and gates |
| 12 | [Source and Evidence Register](12-source-register.md) | Every source, date, quality, limitations |
| 13 | [Reverse Engineering Toolkit and Experimental Research Plan](13-re-toolkit-and-lab.md) | Tools, lab, workflows, environment spec, the "install / study / investigate" lists |
| 14 | [Technical Feasibility Validation Plan](14-feasibility-validation-plan.md) | Targeted audit of the package; platform re-verification; experiments A (iPhone↔Android states), B (multi-hop), C (battery); verified environment; decision gates GA/GB/GS/GC; results tables (all **NOT TESTED**) |
| 15 | [Final Research Audit Report](15-final-audit-report.md) | Checks performed, corrections C-01…C-10, risk→verification traceability, authoritative-document map |
| 16 | [UI/UX Specification](16-ui-ux-specification.md) | Principles, tokens (contrast-verified), navigation, 14 screens, states, accessibility |
| 17 | [Testing and Acceptance Specification](17-testing-acceptance-spec.md) | Automated vs device tests, acceptance criteria, device/OS matrix |
| 18 | [Development Blueprint (authoritative)](18-development-blueprint.md) | Requirements, final architecture (ADR-013…019), APIs, protocol, security, data model, dependencies, roadmap Phases 1–10, final answers, next development prompt |
| 19 | [Development Environment Report](19-development-environment-report.md) | Mac audit, technology re-validation, dependency matrix, installs, verification, manual actions, disk impact |
| 20 | [Development Progress](20-development-progress.md) | **Live status:** phase table, test results, decisions ADR-020…022, known defects, blocked experiments, next steps |
| 21 | [Wire Protocol v1 (normative)](21-wire-protocol-v1.md) | Exact bytes as implemented; supersedes Doc 07 where they differ |
| 22 | [Routing Simulation Results](22-routing-simulation-results.md) | Generated by `vl-sim`; **simulation, not measurement** |
| — | [**Algorithms and optimization**](algorithms/00-index.md) | 45 algorithms evaluated; benchmarks (244× VACCINE speed-up); simulator experiments E1–E4; sync/conflict spec (HLC, ADR-025); DB, BLE, security and architecture reports; roadmap |
| — | [**Messaging-app ecosystem research**](messaging-apps/00-index.md) | 25 app reports (WhatsApp … bitchat, Briar, Meshtastic); feature matrix; architecture, security and offline analyses; algorithm evaluation; feature tiers; roadmap; acceptance criteria; **feature implementation tracker** |

## Evidence levels used throughout

**A** official documentation · **B** reproducible analysis or source code inspected at a pinned commit · **C** credible but unverified · **D** hypothesis requiring experiment.

## Cross-document identifiers

- **ADR-001…012:** Doc 5 §7; **ADR-013…019:** Doc 18; **ADR-020…022:** Doc 18 Addendum A + Doc 20 §4; **ADR-023…024:** Doc 18 Addendum B
- **E1–E17, H1–H8, gates G1–G4:** Doc 8
- **Experiments A/B/C, HA1–HC1, gates GA/GB/GS/GC, probe requirements P-1…P-10:** Doc 14
- **R-01…R-32, G0–G5:** Doc 10
- **T1–T13, S-R1…S-R4:** Doc 6
- **S01…S85:** Doc 12
- **FR/NFR, U-1…U-10, C1–C6:** Doc 18 · **T-PRO/T-CRY/T-DB/T-RTE/T-SEC/T-UI:** Doc 17 · **C-01…C-10:** Doc 15

## Local references used

Shallow clones in `../references/` (OpenHaystack, Briar, Meshtastic firmware + protobufs, Wireshark) were read for code-level evidence. bitchat and google/nearby files were read through the GitHub API. Nothing was executed from any of these repositories.
