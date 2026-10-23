#!/usr/bin/env bash
# Confere a Lavra no Neovim (0.11 ou mais novo), headless, sem plugins de terceiros e sem tocar
# na sua configuração: compila o parser com o cc numa pasta temporária (o scanner com os
# avisos tratados como erro, e em C11, como o Helix o compila) e roda o
# scripts/neovim-check.lua com o plugin de editors/neovim no runtimepath.
#
# Precisa do `nvim` e do `cc` no PATH; com o `lavrac` no PATH, confere também o servidor de
# linguagem.
#
# Uso: scripts/neovim-check.sh

set -euo pipefail

grammar=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
root=$(cd "$grammar/../.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

mkdir -p "$tmp/site/parser"
cc -O2 -fPIC -c -I "$grammar/src" "$grammar/src/parser.c" -o "$tmp/parser.o"
cc -O2 -fPIC -c -std=c11 -Wall -Wextra -Werror -I "$grammar/src" "$grammar/src/scanner.c" -o "$tmp/scanner.o"
cc -shared "$tmp/parser.o" "$tmp/scanner.o" -o "$tmp/site/parser/lavra.so"

nvim --version | sed -n 1p
LAVRA_ROOT="$root" LAVRA_GRAMMAR="$grammar" LAVRA_SITE="$tmp/site" \
  nvim --headless --clean -l "$grammar/scripts/neovim-check.lua"
