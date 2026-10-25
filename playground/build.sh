#!/usr/bin/env bash
# Build the playground: the WebAssembly module and its JS bindings go to
# www/pkg/, and dist/ gets a copy of the page that can be served locally:
#   ./build.sh && python3 -m http.server -d dist 8000
# Needs the wasm32-unknown-unknown target and wasm-bindgen-cli matching the
# wasm-bindgen crate version in Cargo.lock.
set -euo pipefail
cd "$(dirname "$0")"
ROOT=..

cargo build -q -p tt-playground --target wasm32-unknown-unknown --profile wasm --manifest-path "$ROOT/Cargo.toml"
wasm-bindgen --target web --no-typescript --out-dir www/pkg \
  "$ROOT/target/wasm32-unknown-unknown/wasm/tt_playground.wasm"

# The page inlines the generated JS bindings (so it needs no script file
# of its own) and fetches only the .wasm.  dist/page.html is the page body,
# for hosts that add the document skeleton themselves; dist/index.html wraps
# it in one for local serving.
rm -rf dist && mkdir -p dist/pkg
cp www/pkg/tt_playground_bg.wasm dist/pkg/
python3 - www/index.html www/pkg/tt_playground.js dist/page.html <<'PY'
import re, sys
page, glue, out = sys.argv[1:]
js = open(glue).read()
js = re.sub(r"^export function ", "function ", js, flags=re.M)
js = re.sub(r"^export \{[^}]*\};?\s*$", "", js, flags=re.M)
html = open(page).read()
assert "/*@@WASM_BINDGEN_GLUE@@*/" in html
open(out, "w").write(html.replace("/*@@WASM_BINDGEN_GLUE@@*/", js))
PY
python3 - dist/page.html dist/index.html <<'PY'
import sys
page = open(sys.argv[1]).read()
# Everything before the first <header> is head material (title, meta,
# font links, styles); add the small reset the artifact host would supply.
cut = page.index("\n<header")
head, body = page[:cut], page[cut:]
open(sys.argv[2], "w").write(
    '<!doctype html>\n<html lang="en">\n<head>\n<meta charset="utf-8">\n'
    '<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">\n'
    '<style>body { margin: 0 } [hidden] { display: none !important }</style>\n'
    + head + "\n</head>\n<body>" + body + "\n</body>\n</html>\n")
PY
ls -l dist dist/pkg
