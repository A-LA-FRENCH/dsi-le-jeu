#!/usr/bin/env bash
# Découpe chaque dist/pkg-*/dsi_bg.wasm en morceaux d'au plus $MB Mio, pour les
# hébergeurs statiques qui limitent la taille par fichier. Le chargeur les réassemble (parts.json).
#   tools/split-wasm.sh dist 12
set -euo pipefail
DIST="${1:-dist}"; MB="${2:-12}"
for pkg in "$DIST"/pkg-*; do
  [ -f "$pkg/dsi_bg.wasm" ] || continue
  ( cd "$pkg"
    split -b "${MB}M" -d -a 1 --additional-suffix=.wasm dsi_bg.wasm dsi_bg.part
    rm dsi_bg.wasm
    printf '{"parts":[%s]}\n' "$(ls dsi_bg.part*.wasm | sed 's/.*/"&"/' | paste -sd,)" > parts.json
    cat parts.json )
done
