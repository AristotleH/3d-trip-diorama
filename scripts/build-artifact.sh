#!/usr/bin/env bash
# Builds a self-contained copy of web/ for hosts that block outside requests
# (such as a claude.ai artifact): live OpenStreetMap import is compiled out,
# the scene download link is hidden, and pre-built places from
# tools/bake-places.mjs are included.
#
#   scripts/build-artifact.sh [out-dir]   (default: dist/artifact)
set -euo pipefail
cd "$(dirname "$0")/.."
out="${1:-dist/artifact}"

cargo build --locked --release -p diorama-app --target wasm32-unknown-unknown --no-default-features
rm -rf "$out" && mkdir -p "$out"
cp -r web/. "$out/"
rm -rf "$out/pkg" "$out/vendor/README.md"
wasm-bindgen --target web --no-typescript --out-dir "$out/pkg" --out-name diorama_app \
  target/wasm32-unknown-unknown/release/diorama_app.wasm

# The artifact host supplies its own document skeleton, and its sandbox
# blocks downloads.
python3 - "$out/index.html" <<'PY'
import re, sys
path = sys.argv[1]
html = open(path).read()
html = re.sub(r'<!DOCTYPE html>\s*<html[^>]*>\s*<head>\s*<meta charset[^>]*>\s*<meta name="viewport"[^>]*>\s*', '', html)
html = html.replace('</head>\n<body>\n', '').replace('</body>\n</html>', '')
html = html.replace('height: 100vh; height: 100dvh;', 'height: 100%;')
html = html.replace('<style>\n', '<style>\n:root { color-scheme: dark; }\nhtml { height: 100%; background: #191929; }\n', 1)
html = html.replace('<a id="schema"', '<a id="schema" hidden')
open(path, 'w').write(html)
PY
echo "Built $out ($(du -sh "$out" | cut -f1)); $(ls "$out/scenes/places" 2>/dev/null | grep -vc '^index.json$' || true) pre-built places"
