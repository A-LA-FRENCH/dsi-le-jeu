#!/usr/bin/env bash
# Compile la version navigateur de DSI — Le Jeu dans ./dist
#
#   tools/build-web.sh                   # les deux rendus (WebGPU + repli WebGL2)
#   RENDERERS=webgl2 tools/build-web.sh  # un seul (itération plus rapide)
#   PROFILE=dev tools/build-web.sh       # non optimisé, compilation plus rapide
#
# Prérequis : rustup target add wasm32-unknown-unknown
#             cargo install wasm-bindgen-cli --version <celle de Cargo.lock>
# Optionnel : wasm-opt (binaryen natif) dans le PATH pour un binaire plus petit.
#
# Si la cible wasm32 ne peut pas être installée mais que les sources de la std sont
# présentes (composant rust-src), le script compile la std depuis les sources (-Zbuild-std).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

OUT="${OUT:-$ROOT/dist}"
RENDERERS="${RENDERERS:-webgl2 webgpu}"
PROFILE="${PROFILE:-wasm-release}"
TARGET=wasm32-unknown-unknown
BIN=dsi-le-jeu

log() { printf '\033[1;33m==>\033[0m %s\n' "$*"; }
die() { printf '\033[1;31merreur :\033[0m %s\n' "$*" >&2; exit 1; }

lock_ver="$(grep -A1 'name = "wasm-bindgen"$' Cargo.lock | sed -n 's/version = "\(.*\)"/\1/p')"
command -v wasm-bindgen >/dev/null || die "wasm-bindgen introuvable (cargo install wasm-bindgen-cli --version $lock_ver --locked)"
cli_ver="$(wasm-bindgen --version | awk '{print $2}')"
[ "$lock_ver" = "$cli_ver" ] || die "wasm-bindgen-cli $cli_ver ne correspond pas à wasm-bindgen $lock_ver (Cargo.lock)"

SYSROOT="$(rustc --print sysroot)"
BUILD_STD=()
if [ -d "$SYSROOT/lib/rustlib/$TARGET/lib" ]; then
  log "Bibliothèque standard $TARGET installée"
elif [ -d "$SYSROOT/lib/rustlib/src/rust/library/std" ]; then
  log "std $TARGET absente : compilation depuis rust-src (-Zbuild-std)"
  export RUSTC_BOOTSTRAP=1
  BUILD_STD=(-Zbuild-std=std,panic_abort)
else
  die "Lancez : rustup target add $TARGET"
fi

case "$PROFILE" in
  dev) PROFILE_DIR=debug ;;
  release) PROFILE_DIR=release ;;
  *) PROFILE_DIR="$PROFILE" ;;
esac

rm -rf "$OUT"
mkdir -p "$OUT"

for r in $RENDERERS; do
  case "$r" in webgl2|webgpu) ;; *) die "rendu inconnu $r" ;; esac
  log "Compilation ($r, profil $PROFILE)…"
  cargo build -p dsi-client --bin "$BIN" --target "$TARGET" --profile "$PROFILE" \
    --no-default-features --features "$r" ${BUILD_STD[@]+"${BUILD_STD[@]}"}
  wasm="target/$TARGET/$PROFILE_DIR/$BIN.wasm"
  log "wasm-bindgen → pkg-$r"
  wasm-bindgen --target web --no-typescript --out-dir "$OUT/pkg-$r" --out-name dsi "$wasm"
  if command -v wasm-opt >/dev/null && [ "$PROFILE" != dev ]; then
    log "wasm-opt -Oz"
    wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext \
      --enable-mutable-globals --enable-reference-types --enable-multivalue \
      "$OUT/pkg-$r/dsi_bg.wasm" -o "$OUT/pkg-$r/dsi_bg.opt.wasm"
    mv "$OUT/pkg-$r/dsi_bg.opt.wasm" "$OUT/pkg-$r/dsi_bg.wasm"
  fi
done

# Si un seul rendu a été compilé, WebGL2 sert aussi de build « WebGPU » (le chargeur
# essaie WebGPU d'abord ; WebGL2 fonctionne partout où WebGPU fonctionne).
if [ ! -d "$OUT/pkg-webgpu" ] && [ -d "$OUT/pkg-webgl2" ]; then
  cp -r "$OUT/pkg-webgl2" "$OUT/pkg-webgpu"
fi

log "Copie de la page web et des ressources"
cp web/index.html web/style.css web/loader.js web/favicon.png "$OUT/"
[ -f web/_headers ] && cp web/_headers "$OUT/"
mkdir -p "$OUT/assets"
for d in sprites ui fonts audio; do
  cp -r "assets/$d" "$OUT/assets/"
done
log "Terminé → $OUT"
du -sh "$OUT" "$OUT"/pkg-* 2>/dev/null
