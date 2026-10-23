#!/usr/bin/env bash
# Confere a Lavra no Helix com a gramática desta árvore, sem tocar na sua configuração:
#
# - numa configuração temporária, o próprio Helix compila a gramática de src/ (com o
#   compilador de C++ do PATH, como faz com qualquer gramática);
# - o `hx --health lavra` tem de mostrar o parser carregado e as três queries no lugar;
# - um programa do corpus abre num tmux, ganha uma linha nova num bloco (a query de recuo) e
#   seleciona a função em volta (a de objetos de texto). O health só confere que os arquivos
#   existem; quem compila as queries é o uso, e o erro de uma query que não compila vai para
#   o log do Helix, que não pode ter erro.
#
# Precisa do `hx` (25.07 ou mais novo), do git (o Helix o exige até para compilar uma gramática
# local), de um compilador de C++ e do tmux no PATH. Com o `lavrac` no PATH, o servidor de
# linguagem e o depurador também aparecem no health. Com HELIX_CAPTURE=arquivo, a tela do
# Helix com o programa aberto vai para o arquivo, com as cores (sequências ANSI).
#
# Uso: scripts/helix-health.sh

set -euo pipefail

grammar=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
root=$(cd "$grammar/../.." && pwd)
tmp=$(mktemp -d)
socket="$tmp/tmux.socket"
trap 'tmux -S "$socket" kill-server 2>/dev/null || true; rm -rf "$tmp"' EXIT

export XDG_CONFIG_HOME="$tmp/config" XDG_CACHE_HOME="$tmp/cache" XDG_DATA_HOME="$tmp/data" LC_ALL=C.UTF-8
config="$XDG_CONFIG_HOME/helix"
mkdir -p "$config/runtime/queries" "$config/runtime/grammars"

# A configuração do repositório, com a gramática desta árvore no lugar do git, e só ela para
# compilar.
{
  echo 'use-grammars = { only = ["lavra"] }'
  sed "s|^source = { git = .*|source = { path = \"$grammar\" }|" "$root/editors/helix/languages.toml"
} >"$config/languages.toml"
cp -r "$root/editors/helix/runtime/queries/lavra" "$config/runtime/queries/"
# Sem o lavrac, o Helix registraria no log a falha de iniciar o servidor, que não é o que se
# confere aqui: a sessão fica sem servidor.
if ! command -v lavrac >/dev/null; then
  echo "Sem o lavrac no PATH: o servidor de linguagem e o depurador ficam de fora."
  sed -i 's/^language-servers = \["lavrac"\]/language-servers = []/' "$config/languages.toml"
fi

hx --version
hx --grammar build
health=$(NO_COLOR=1 hx --health lavra 2>&1 | sed 's/\x1b\[[0-9;]*m//g')
echo "$health"

problems=0
for item in 'Tree-sitter parser' 'Highlight queries' 'Textobject queries' 'Indent queries'; do
  if ! grep -q "^$item: ✓" <<<"$health"; then
    echo "O health não mostra \"$item: ✓\"." >&2
    problems=1
  fi
done

# O programa aberto: a linha de baixo de "Para começar:" está no bloco principal.
program="$tmp/catilinaria.lav"
cp "$root/samples/catilinaria.lav" "$program"
line=$(grep -n -m1 -i '^para começar:' "$program" | cut -d: -f1)
tmux -S "$socket" new-session -d -x 110 -y 60 "hx '$program'"
sleep 3
if [ -n "${HELIX_CAPTURE:-}" ]; then
  tmux -S "$socket" capture-pane -e -p >"$HELIX_CAPTURE"
fi
tmux -S "$socket" send-keys "$((line + 1))G" o
# O Escape vai sozinho: colado na tecla seguinte, o terminal o lê como Alt.
sleep 0.5
tmux -S "$socket" send-keys Escape
sleep 0.5
tmux -S "$socket" send-keys m a f
sleep 0.5
tmux -S "$socket" send-keys Escape
sleep 0.5
tmux -S "$socket" send-keys u ':q!' Enter
for _ in $(seq 20); do
  tmux -S "$socket" has-session 2>/dev/null || break
  sleep 0.5
done
# Se o Helix não saiu, as teclas não chegaram, e as queries de recuo e de objetos de texto
# podem não ter sido usadas.
if tmux -S "$socket" has-session 2>/dev/null; then
  echo "O Helix não saiu com :q!; a tela era:" >&2
  tmux -S "$socket" capture-pane -p >&2
  problems=1
fi

# Os erros do transporte do servidor de linguagem não contam: no fechamento, depois do
# "shutdown" respondido e do "exit", o Helix sempre registra o canal e o stdout do servidor
# que fecharam. O que este roteiro procura são as queries que não compilam.
log="$XDG_CACHE_HOME/helix/helix.log"
errors=$(grep '\[ERROR\]' "$log" 2>/dev/null | grep -v ' helix_lsp::transport \[ERROR\]' || true)
if [ -n "$errors" ]; then
  echo "O log do Helix tem erros:" >&2
  echo "$errors" >&2
  problems=1
fi
if [ "$problems" -eq 0 ]; then
  echo "Helix: parser e queries carregados, programa aberto sem erro no log."
fi
exit "$problems"
