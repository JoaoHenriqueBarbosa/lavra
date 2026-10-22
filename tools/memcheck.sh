#!/usr/bin/env bash
# Compila cada programa da suíte para nativo e roda sob valgrind. Usa o valgrind do
# sistema quando ele existe (o CI instala); senão, um contêiner efêmero, para não
# instalar nada no host.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$(mktemp -d)"
trap 'rm -rf "$OUT"' EXIT
cargo build -q --manifest-path "$ROOT/Cargo.toml"
for f in "$ROOT"/tests/programs/ok/*.lav "$ROOT"/tests/programs/err/*.lav "$ROOT"/samples/*.lav; do
  name="$(basename "$(dirname "$f")")_$(basename "$f" .lav)"
  if (cd "$ROOT" && ./target/debug/lavrac build "${f#"$ROOT"/}" -o "$OUT/$name" 2>/dev/null); then
    if [ -f "${f%.lav}.stdin" ]; then cp "${f%.lav}.stdin" "$OUT/$name.stdin"; fi
  fi
done
if command -v valgrind >/dev/null 2>&1; then
  bash "$ROOT/tools/valgrind-all.sh" "$OUT"
else
  docker run --rm -v "$OUT":/w -v "$ROOT/tools":/t:ro debian:trixie-slim bash -c '
    apt-get update -qq >/dev/null && apt-get install -y -qq valgrind >/dev/null 2>&1
    bash /t/valgrind-all.sh /w'
fi
