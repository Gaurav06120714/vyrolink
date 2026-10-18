# VYRO LINK

Offline, end-to-end encrypted, store-and-forward messaging over Bluetooth LE (iOS + Android).

> **Research and design documents** (`research/`, `references/`) were moved out of this project on 2026-10-18 to `../VyroLink-research/`. Links below that start with `research/` point there.

**Status (2026-10-16):**
- **Done:** the shared Rust core is implemented and tested on this Mac:
  - 137 tests, fuzzed, Swift interop passing;
  - Tier 1 conversation features: replies, reactions, edit/delete, search, drafts, blocking, disappearing messages, panic wipe;
  - small groups (≤ 8 members, one encrypted copy per member);
  - small attachments (images ≤ 16 KiB, files ≤ 64 KiB), a 32-character My ID code with QR, contact rename/delete/re-pair, resend.
- **iOS app:** the Flutter app with its native Swift host runs on the iOS 27 simulator against the real Rust core:
  - it has onboarding, QR pairing, My ID, chats, conversations, groups, attachments, search, delivery details, network, relay settings, privacy with app lock, settings and panic wipe;
  - 29 Flutter tests pass, plus an end-to-end simulator test.
- **Android app:** compiles (Kotlin host + Rust core for arm64) and runs on an Android 37 emulator; the same end-to-end test passes there. It adds the Off / While charging / Always relay modes (foreground service).
- **Blocked:** Bluetooth between physical phones (no Android device; the iPhone is offline). Nothing here is device-validated.
- **Not production-ready.**

- **[SETUP.md](SETUP.md)**: setup, build, test and verification. This is the single source of truth.
- **[research/20-development-progress.md](research/20-development-progress.md)**: live status, results, defects, blocked experiments.
- **[research/00-INDEX.md](research/00-INDEX.md)**: research and design package (Docs 01–22; Doc 18 is the authoritative blueprint; Doc 21 is the wire protocol).
- **[research/messaging-apps/](research/messaging-apps/00-index.md)**: messaging-ecosystem research and the feature tracker.
- **[research/algorithms/](research/algorithms/00-index.md)**: algorithm evaluation, benchmarks and routing experiments.
- **[research/FEATURE-CHECKLIST.md](research/FEATURE-CHECKLIST.md)**: every researched feature and requirement ticked against the built iOS and Android apps.
- **[research/RESEARCH.md](research/RESEARCH.md)**: notes on the Claude Code skills and reference repos.
- **[website/](website/index.html)**: the one-page product site with the privacy policy, hosted on Netlify (`netlify.toml`).

```
VyroLink/
├── README.md / SETUP.md
├── research/              ← ALL research in one place: Docs 00–22, messaging-apps/, algorithms/, RESEARCH.md, FEATURE-CHECKLIST.md
├── app/                   ← Flutter app (lib/) + packages/vl_host: native iOS (Swift) and Android (Kotlin) host plugin
├── core/                  ← Rust workspace: vl-proto, vl-crypto, vl-routing, vl-store, vl-core, vl-ffi, vl-sim (+ fuzz/, vectors/, bindings-tests/)
├── tools/                 ← build-core-ios.sh, build-core-android.sh, android-sdk-setup.sh, generate_icons.py, swift-interop-test.sh
├── website/               ← one-page site + privacy policy (static, no build; Netlify publishes this folder)
├── netlify.toml           ← Netlify config: publish website/, security headers
├── .github/workflows/     ← CI for core/ (runs on push to GitHub)
├── .mcp.json              ← REA MCP server (rea-agents@6.5.0), project scope
└── references/            ← read-only reference codebases, git-ignored, not in the GitHub repo (shallow clones, all GPL/AGPL: study, don't copy)
    ├── seemoo-lab-openhaystack/   Find My network RE (+ Flutter app in openhaystack-mobile/)
    ├── briar-briar/               offline P2P messenger (Bluetooth / Wi-Fi / Tor)
    ├── meshtastic-firmware/       LoRa mesh firmware (routing, crypto, BLE phone API)
    ├── meshtastic-protobufs/      Meshtastic wire protocol (.proto)
    └── wireshark-wireshark/       protocol analyzer source (BLE / LoRa / protobuf dissectors)
```

## Skills by area

The 82 Claude Code skills and the `/extract-ipa` command were moved out of this project on 2026-10-15. They now live in the user folder, `~/.claude/skills/` and `~/.claude/commands/`, and are available in every project.

| Area | Skills |
|---|---|
| Flutter (official) | `flutter-add-integration-test` `flutter-add-widget-preview` `flutter-add-widget-test` `flutter-apply-architecture-best-practices` `flutter-build-responsive-layout` `flutter-fix-layout-issues` `flutter-implement-json-serialization` `flutter-setup-declarative-routing` `flutter-setup-localization` `flutter-use-http-package` |
| Dart (official) | `dart-add-unit-test` `dart-build-cli-app` `dart-collect-coverage` `dart-fix-runtime-errors` `dart-generate-test-mocks` `dart-migrate-to-checks-package` `dart-resolve-package-conflicts` `dart-run-static-analysis` `dart-setup-ffi-assets` `dart-use-doc-examples` `dart-use-ffigen` `dart-use-path-package` `dart-use-pattern-matching` `dart-use-primary-constructors` `dart-write-documentation` |
| Flutter engineering (thiennc, 37) | start with `flutter-app-workflow`; also `flutter-architecture` `flutter-state-management` `flutter-background-execution` `flutter-platform-integration` `flutter-security` `flutter-persistence` `flutter-networking` `flutter-testing` `flutter-device-testing` `flutter-performance` … |
| Clean Architecture (gabuldev, 15) | `flutter-clean-arch` `flutter-new-module` `flutter-state` `flutter-di` `flutter-monorepo` `flutter-caching` `flutter-concurrency` `flutter-testing-gabul` `flutter-accessibility-gabul` … |
| Testing automation | `flutter-autonomous` |
| Design | `mobile-uiux-pro-max` `mobile-flutter-design` |
| Reverse engineering | `ios-reverse-engineering` (+ `/extract-ipa`) `reverse-engineer-anything` (+ REA MCP) |

Open Claude Code in this folder to use them. Run `/skills` to list them; Claude also picks one automatically when a task matches its description.

## Development rules

- The app was authorised by you on 2026-10-15 ("build the real Flutter mobile application").
- Claims about real Bluetooth relaying (background behaviour, cross-platform, multi-hop) still wait for the device-experiment gates GA/GB/GS ([Doc 14](research/14-feasibility-validation-plan.md)).
- Before calling any change done, run the checklist in [SETUP.md](SETUP.md) §9.
