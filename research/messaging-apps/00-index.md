# Messaging-App Ecosystem Research — Index

**Created:** 2026-10-15 for VYRO LINK. This is part of the research package; see `../00-INDEX.md`.

**Method:**
- Current web sources: official documentation, papers, audits and reporting.
- **Source inspection** of open-source apps at pinned commits:
  - bitchat iOS `5e9287f` and Android `6ade682`;
  - Delta Chat core `64e5875`;
  - SimpleX `479548e`;
  - Briar `d03d6fc`;
  - Meshtastic protobufs `28f4f5f` and firmware `f7f66b4`.

  Clones live in `../../references/` (git-ignored). The code was studied only; nothing was copied.
- **REA (reverse-engineer-anything) was applied to VYRO LINK's own artifacts.**
  - It inspected the iOS `libvl_ffi.dylib`: no networking imports. It also flagged `os_log`, which led to the SQLCipher logging fix.
  - It inspected the test databases: REA cannot open them, which is independent evidence of encryption at rest.
  - It was **not** applied to proprietary apps. WhatsApp and Messages are installed on this Mac, but their licence terms prohibit reverse engineering, and the task restricts us to lawful methods.

**Evidence labels:**
- **VERIFIED:** primary source, or code at a pinned commit.
- **REPORTED:** credible secondary source.
- **INFERRED:** our analysis.
- **UNKNOWN:** not established.

Every app report lists its sources with access dates and confidence.

## Per-application reports (`apps/`)

**Mainstream:**
[WhatsApp](apps/whatsapp.md) · [Telegram](apps/telegram.md) · [Instagram Direct](apps/instagram-direct.md) · [Facebook Messenger](apps/facebook-messenger.md) · [Signal](apps/signal.md) · [Discord](apps/discord.md) · [iMessage](apps/imessage.md) · [Google Messages](apps/google-messages.md) · [Snapchat](apps/snapchat.md) · [Viber](apps/viber.md) · [LINE](apps/line.md) · [WeChat](apps/wechat.md)

**Privacy and federated:**
[Element/Matrix](apps/element-matrix.md) · [SimpleX Chat](apps/simplex-chat.md) · [Session](apps/session.md) · [Threema](apps/threema.md) · [Wire](apps/wire.md) · [Delta Chat](apps/delta-chat.md)

**Offline, mesh and delay-tolerant:**
[Briar](apps/briar.md) · [bitchat](apps/bitchat.md) · [Bridgefy](apps/bridgefy.md) · [Meshtastic](apps/meshtastic.md) · [Berty](apps/berty.md) · [Reticulum/Sideband](apps/reticulum-sideband.md) · [FireChat (historical)](apps/firechat.md)

## Cross-application analysis

| # | Document | Answers |
|---|---|---|
| 01 | [Feature comparison matrix](01-feature-comparison-matrix.md) | Who has what, and how verified |
| 02 | [Architecture comparison](02-architecture-comparison.md) | Families, ordering authority, patterns to adopt |
| 03 | [Security and privacy analysis](03-security-privacy-analysis.md) | Attack lessons, metadata, feature risks, new risks |
| 04 | [Offline-messaging analysis](04-offline-messaging-analysis.md) | What offline changes for each feature; cost model |
| 05 | [Technology and algorithm evaluation](05-technology-algorithm-evaluation.md) | Routing, dedup, sync, event model, search, groups, media, versioning |
| 06 | [Feature prioritization](06-feature-prioritization.md) | Tier 1/2/3, deferred, rejected, with scores |
| 07 | [Implementation roadmap](07-implementation-roadmap.md) | Order and dependencies |
| 08 | [Feature acceptance criteria](08-feature-acceptance-criteria.md) | Testable criteria for every feature |
| 09 | [Feature implementation tracker](09-feature-implementation-tracker.md) | **Live status** |

## Headline conclusions

1. **The architecture holds up.** Rust core + bindings (ADR-013/022), spray-and-wait L = 8 (ADR-021) and vaccine-as-proof (ADR-020) are consistent with the evidence. bitchat independently converged on spray-and-wait with 8 copies and rotating tags. **No ADR changes.**
2. **Rich features work offline if each change is an authenticated event that references a message ID, merged order-independently** (Delta Chat, SimpleX, Matrix). New ADR-023 records this (Doc 18).
3. **MLS and server-ordered designs do not fit an offline network** (commit ordering, forks). Groups use pairwise fan-out first.
4. **Real-time calls, presence, cloud sync and phone-number discovery are rejected.**
5. **Two real security issues were found and fixed** during this research (SQLCipher device logging; low-order keys in cards).
