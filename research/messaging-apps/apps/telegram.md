# Telegram — research report

**Accessed:** 2026-10-15. **Method:** public papers and documentation. Client source is open (GPL), but the server is closed. **Confidence:** Medium.

## 1. Capabilities

| Capability | Behaviour | Label |
|---|---|---|
| Default chats | **Cloud chats**: the server can read them (not E2EE) | REPORTED (widely documented) |
| Secret chats | Opt-in E2EE using MTProto 2.0, 1:1 only, single device | REPORTED |
| Formal analysis | ProVerif analysis (Udine, SECRYPT 2021, revised 2023): authentication, secret-chat encryption and rekeying hold for the properties tested, including PFS. The **rekeying protocol is vulnerable to an unknown key-share attack**; the computational model is still open | REPORTED (paper abstract) |
| Earlier attacks | Albrecht et al., IEEE S&P 2022: attacks on MTProto implementations. ETH: replay/timing issues in third-party clients | REPORTED |
| Edit | ~48 h (API docs say "usually") | REPORTED |
| Channels, huge groups, bots, polls, reactions, scheduled messages, folders | Yes | REPORTED |

## 2. Lessons

Feature-rich, but its privacy model differs from ours. **Reject** the "cloud by default" model.

**Adopt:** scheduled sends (Tier 3 for offline: "send when a carrier is nearby" is already the default behaviour).

## 3. Sources (accessed 2026-10-15)

- https://air.uniud.it/handle/11390/1210973 (medium–high)
- https://www.research-collection.ethz.ch/bitstream/handle/20.500.11850/620789/telegram_client_analysis.pdf (high)
- https://arxiv.org/pdf/2012.03141v1 (medium)
- https://green-api.com/en/telegram/docs/api/service/EditMessage (low–medium)
