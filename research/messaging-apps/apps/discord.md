# Discord — research report

**Accessed:** 2026-10-15. **Confidence:** Medium.

| Item | Finding | Label |
|---|---|---|
| Text messages | **Not E2EE**, and there are no plans to add it | REPORTED (The Hacker News 2024-09) |
| DAVE (voice/video E2EE) | WebRTC encoded transforms for frame encryption + **MLS** for group key exchange; per-sender frame keys; forward secrecy and post-compromise security on membership change; optional persistent or ephemeral identity keys; out-of-band verification; audited by Trail of Bits; protocol and library published; fallbacks to non-E2EE were being removed | REPORTED |
| Features | Servers/channels, threads, reactions, replies, pins, search, edit (no time limit reported) | REPORTED |

**Lesson:** a real-time media E2EE design depends on a live SFU and continuous connectivity, so it is **not applicable offline**. Real-time calls are **rejected** for VYRO LINK. BLE goodput and intermittency cannot support them; see 06.

## Sources (accessed 2026-10-15)

- https://thehackernews.com/2024/09/discord-introduces-dave-protocol-for.html (medium)
- https://thestack.technology/discord-encryption-mls-e2ee (medium)
