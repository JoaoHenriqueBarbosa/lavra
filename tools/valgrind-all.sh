#!/usr/bin/env bash
# Roda cada executável de uma pasta sob valgrind, com o .stdin ao lado como entrada.
# Acusa vazamento e acesso inválido. Usado por tools/memcheck.sh, no host ou no contêiner.
set -uo pipefail
DIR="$1"
bad=0
n=0
for exe in "$DIR"/*; do
  case "$exe" in *.stdin|*.vg) continue ;; esac
  n=$((n + 1))
  in=/dev/null
  [ -f "$exe.stdin" ] && in="$exe.stdin"
  valgrind --leak-check=full --errors-for-leak-kinds=definite,indirect --error-exitcode=99 -q "$exe" <"$in" >/dev/null 2>"$exe.vg"
  if [ $? -eq 99 ]; then
    bad=$((bad + 1))
    echo "PROBLEMA: $(basename "$exe")"
    grep -E "lost|Invalid" "$exe.vg" | head -5
  fi
  rm -f "$exe.vg"
done
echo "$n executáveis conferidos, $bad com problema de memória"
[ "$bad" -eq 0 ]
