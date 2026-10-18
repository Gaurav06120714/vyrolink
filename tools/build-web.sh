#!/usr/bin/env bash
# Builds the web chat (ADR-040) into app/build/web, ready to host as a static site.
# Usage: [VL_SCRIPTS=https://site/chat/] tools/build-web.sh https://your-mailbox.onrender.com [extra flutter args]
# VL_SCRIPTS is where the chat is served; scripts from anywhere else are refused
# (default 'self', for local testing).
# Everything is served from the site itself (no CDN); the page's
# Content-Security-Policy allows connections only to that mailbox.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MAILBOX="${1:?usage: tools/build-web.sh <mailbox URL>}"; shift
"$ROOT/tools/build-core-web.sh"
cd "$ROOT/app"
flutter build web --release --no-web-resources-cdn --dart-define=VL_MAILBOX="$MAILBOX" "$@"
sed -i '' -e "s#__VL_MAILBOX__#$MAILBOX#" -e "s#__VL_SCRIPTS__#${VL_SCRIPTS:-'self'}#" build/web/index.html
grep -q "__VL_" build/web/index.html && { echo "CSP placeholder left in index.html"; exit 1; }
echo "Built app/build/web for mailbox $MAILBOX"
