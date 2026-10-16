# 01 — Feature Comparison Matrix

**Accessed:** 2026-10-15. Each cell is backed by the per-app report in `apps/`.

**Key:**
- ✓ present
- ✗ absent
- ~ partial or optional
- ? unknown
- **bold** = verified in source or an official spec (VERIFIED); the rest are REPORTED

## 1. Messaging features

| App | 1:1 E2EE default | Groups | Reactions | Replies | Edit (window) | Delete-for-all (window) | Disappearing | Receipts | Voice notes | Attachments | Search | Pins | Drafts | Polls | Channels |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| WhatsApp | **✓** | ✓ | ✓ | ✓ | **✓ 15 min** | ✓ ~60 h (conflicting) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Telegram | ✗ (cloud) / ~ secret chats | ✓ | ✓ | ✓ | ✓ ~48 h | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Instagram DM | ✗ (E2EE removed 2026-05-08) | ✓ | ✓ | ✓ | ✓ | ✓ | ~ vanish | ✓ | ✓ | ✓ | ✓ | ? | ? | ? | ~ |
| Messenger | **✓** (Dec 2023) | ✓ (group E2EE ?) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ? | ? | ✓ | ✗ |
| Signal | **✓** | ✓ | ✓ | ✓ | **✓ 24 h, ≤ 10 edits** | ✓ 24 h (official poll page) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ ≤ 3, timed | ✓ | **✓** | ✗ |
| Discord | ✗ text / ✓ calls (DAVE) | ✓ | ✓ | ✓ | ✓ | ✓ | ✗ | ✗ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| iMessage | ✓ (PQ3) | ✓ | ✓ tapbacks | ✓ | ✓ 15 min, ≤ 5 | ✓ unsend 2 min | ✗ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ? | ✗ |
| Google Messages | ✓ (MLS cross-platform beta) | ✓ | ✓ | ✓ | ✓ | ✓ | ? | ✓ | ✓ | ✓ | ✓ | ? | ✓ | ? | ✗ |
| Snapchat | ~ (snaps only) | ✓ | ✓ | ✓ | ? | ✓ | ✓ default | ✓ | ✓ | ✓ | ~ | ? | ? | ? | ~ |
| Viber | ✓ (vendor claim) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ? | ✓ | ✓ |
| LINE | ✓ Letter Sealing v2 | ✓ | ✓ | ✓ | ? | ✓ unsend | ? | ✓ | ✓ | ✓ | ✓ | ✓ | ? | ✓ | ✓ |
| WeChat | ✗ | ✓ | ✓ | ✓ | ? | ✓ recall | ✗ | ? | ✓ | ✓ | ✓ | ✓ | ? | ? | ✓ |
| Element/Matrix | ✓ (Olm/Megolm, vodozemac) | ✓ | ✓ | ✓ | ✓ | ✓ redaction | ~ | ✓ | ✓ | ✓ | ~ | ✓ | ✓ | ✓ | ✓ rooms |
| Briar | **✓** | **✓** private groups / forums | ✗ | ? | ✗ | ✗ | **✓ 1 min–365 d** | ✓ | ✗ | **✓ images ≤ 32 KiB** | ? | ✗ | ? | ✗ | ✓ blogs |
| SimpleX | ✓ | **✓** | **✓ 8 fixed emoji** | ✓ | **✓ live** | **✓ (pref)** | **✓ (pref)** | ✓ | ✓ | ✓ XFTP | ✓ | ? | ✓ | ? | ~ |
| Session | ✓ (PFS restored in V2) | ✓ | ✓ | ✓ | ? | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ? | ? | ✗ | ✓ communities |
| Threema | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ? | ✓ | ✓ | ✓ | ✓ | ? | ✓ | ✓ | ✗ |
| Wire | ✓ (MLS) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ? | ? | ? | ✗ |
| bitchat | **✓ private** / ✗ public | **✓ creator-signed** | ✗ | ✗ | ✗ | ✗ | ~ ephemeral by default | **✓ delivered + read** | **✓ files + live PTT** | **✓** | ? | ✗ | ? | ✗ | ✓ geohash boards |
| Bridgefy | ~ (broken twice) | ✓ broadcast | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ✓ broadcast |
| Meshtastic | ~ (PKC DMs since 2.5) | ✓ channels | **✓ tapback emoji flag** | **✓ reply_id** | ✗ | ✗ | ✗ | ~ ACK | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✓ channels |
| Berty | ✓ | ✓ | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? | ? |
| Delta Chat | **✓** | **✓** | **✓ (XEP-0444 semantics)** | ✓ | **✓ sender-only** | **✓ tombstones** | **✓ timer from seen** | ✓ | ✓ | ✓ | ✓ | **✓** | ✓ | ? | ✓ |
| Reticulum/Sideband | ✓ | ~ | ? | ? | ? | ? | ? | ✓ | ✓ | ✓ | ? | ? | ? | ? | ✗ |
| FireChat (defunct) | ✗ → ~ | ✓ public | ? | ? | ? | ? | ? | ? | ? | ~ | ? | ? | ? | ? | ✓ public |

## 2. Identity, discovery and verification

| App | Identifier | Contact discovery | QR pairing | Key verification | Multi-device |
|---|---|---|---|---|---|
| WhatsApp / Signal / Viber | Phone number (Signal also usernames) | Address book | QR for linking devices | Safety numbers | Yes (server) |
| Telegram | Phone / username | Address book, username | Login QR | Key image (secret chats) | Yes (cloud) |
| iMessage | Apple Account / phone | Address book | ✗ | Contact Key Verification | Yes |
| Threema | Random 8-char ID | Optional address-book hash | **✓ (green level)** | **3 levels** | Desktop + Android (2025) |
| Briar | Keypair | **In person / link** | **✓** | Implicit (in person) | ✗ |
| SimpleX | **None (one-way queues)** | Links / QR | ✓ | Security code | ~ (desktop link) |
| Session | Public key | Account ID | ✓ | ? | ✓ (config messages) |
| bitchat | Noise static key fingerprint | **Nearby announce** | **✓ verify challenge** | QR + "vouch" web of trust | ✗ |
| Meshtastic | Node number + key | Mesh announce | ✓ channel QR | TOFU | ✗ |
| Delta Chat | Email address | Email / QR SecureJoin | **✓** | SecureJoin | ✓ |
| **VYRO LINK** | **Pairwise only, no global ID** | **In-person QR only** | **✓ signed card** | **Safety number (12×5 digits)** | **✗ (deferred)** |

## 3. Reading the matrix

- Every mainstream app depends on servers for **edit, delete, multi-device and search sync**. None of those survive offline unchanged.
- The **offline-capable** apps (Briar, bitchat, Meshtastic, Reticulum) have far fewer conversational features.
- **Delta Chat and SimpleX** show that rich features work over *asynchronous, out-of-order* transports when each change is a **small authenticated event referencing a message ID**.
