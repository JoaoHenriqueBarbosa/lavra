#!/usr/bin/env bash
# Compila cada programa da suíte para nativo e roda sob valgrind num contêiner
# efêmero (o host não tem valgrind). Acusa vazamento e acesso inválido.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$(mktemp -d)"
cargo build -q --manifest-path "$ROOT/Cargo.toml"
for f in "$ROOT"/tests/programs/ok/*.lav "$ROOT"/tests/programs/err/*.lav "$ROOT"/samples/*.lav; do
  name="$(basename "$(dirname "$f")")_$(basename "$f" .lav)"
  if (cd "$ROOT" && ./target/debug/lavrac build "${f#$ROOT/}" -o "$OUT/$name" 2>/dev/null); then
    [ -f "${f%.lav}.stdin" ] && cp "${f%.lav}.stdin" "$OUT/$name.stdin"
  fi
done
docker run --rm -v "$OUT":/w -v "$ROOT":"$ROOT" -w "$ROOT" debian:trixie-slim bash -c '
  apt-get update -qq >/dev/null && apt-get install -y -qq valgrind >/dev/null 2>&1
  bad=0; n=0
  for exe in /w/*; do
    case "$exe" in *.stdin|*.vg) continue;; esac
    n=$((n+1)); in=/dev/null; [ -f "$exe.stdin" ] && in="$exe.stdin"
    set +e
    valgrind --leak-check=full --errors-for-leak-kinds=definite,indirect --error-exitcode=99 -q "$exe" <"$in" >/dev/null 2>"$exe.vg"
    code=$?
    set -e
    if [ $code -eq 99 ]; then bad=$((bad+1)); echo "PROBLEMA: $(basename "$exe")"; grep -E "lost|Invalid" "$exe.vg" | head -5; fi
  done
  echo "$n executáveis conferidos, $bad com problema de memória"
  rm -f /w/*.vg
  [ $bad -eq 0 ]'
rm -rf "$OUT"
