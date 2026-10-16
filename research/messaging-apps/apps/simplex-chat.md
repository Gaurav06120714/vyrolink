# SimpleX Chat — research report

**Accessed:** 2026-10-15. **Source code inspected:** `simplex-chat/simplex-chat` @ `479548e` (2026-10-03, AGPL-3.0, sparse checkout of `src/Simplex/Chat`; studied only).

**Overall confidence:** High for the protocol events cited; medium for audit details.

## 1. Why it matters

SimpleX has **no user identifiers**: it uses one-way queues on relay servers. Its *chat-level* protocol cleanly separates events (`x.msg.new/update/del/react`) and **negotiates feature preferences per contact**. Both ideas transfer directly to an offline design.

## 2. Capabilities (VERIFIED in `Protocol.hs`, `Types/Preferences.hs` unless noted)

| Capability | Status | Detail |
|---|---|---|
| Edit | `XMsgUpdate {msgId, content, ttl, live, …}` | "live" messages are edited as you type |
| Delete | `XMsgDel {msgId, memberId?, onlyHistory}`; a "full delete" preference decides between removal and a "marked deleted" placeholder | `fullDelete` preference |
| Reactions | `XMsgReact {msgId, memberId?, reaction, add: Bool}` — explicit add/remove toggle | — |
| Reaction set | **Fixed allowlist of 8 emoji:** 👍 👎 😀 😂 😢 ❤️ 🚀 ✅. Unknown emoji parse as `MRUnknown` and display as "?" (forward-compatible) | `Protocol.hs` line ~623 |
| Disappearing ("timed") messages | `ttl` on messages; per-contact `timedMessages` preference | — |
| Per-contact preferences | timedMessages, fullDelete, reactions, voice, files, calls (contacts); plus directMessages, history, reports, signMessages (groups). **Both sides must allow** | `Preferences.hs` |
| Groups | Member introductions (`x.grp.mem.intro/inv/fwd`) build a full mesh of pairwise connections; roles; restrictions | `x.grp.mem.*` events |
| Files | XFTP (separate file relays, chunked) | REPORTED (docs) |
| Audits | Trail of Bits: 2022 implementation review (2 medium, 2 low); 2024 protocol design review, "no critical issues" | REPORTED (Trail of Bits library; SimpleX blog) |

## 3. Lessons

**Adopt:**
- **per-contact negotiated preferences**, e.g. disappearing messages and full delete only when both sides allow;
- **reactions as add/remove toggles**;
- a **small fixed reaction set**: this bounds size, keeps parsing simple, and keeps unknown values forward-compatible;
- a distinct "deleted" placeholder rather than silent disappearance.

**Avoid:** relay servers and XFTP (internet).

## 4. Sources (accessed 2026-10-15)
- Source: `references/simplex-chat` @ 479548e, `src/Simplex/Chat/Protocol.hs`, `src/Simplex/Chat/Types/Preferences.hs`. High confidence.
- https://trailofbits.com/library/simplex-chat/ (medium).
- SimpleX blog "v6.1 security review" (mirror): https://forgejo.multed.com/android-apps-mirrors/SimpleX-Chat/src/branch/master/blog/20241014-simplex-network-v6-1-security-review-better-calls-user-experience.md (medium).
