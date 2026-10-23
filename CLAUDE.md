# Lavra

Linguagem de programação de baixo nível e compilada em que cada instrução é uma
oração em português, e a gramática é verificação: artigo marca variável, modo verbal
marca o papel da oração, regência marca os argumentos, concordância é checagem
redundante. Extensão `.lav`.

## Estado

O compilador em Rust (`src/`, `runtime/`, crate `lavrac`) implementa a Lavra, "um Rust
com gramática e morfologia": interpretador e nativo (Cranelift), posse e empréstimo,
talvez e falha, registros e enumerações. Programas de exemplo e poemas em `samples/`.
Extensão do VS Code em `editors/vscode`; a gramática tree-sitter e as configurações do Helix,
do Neovim e do Zed nas outras pastas de `editors/`.

**A Lavra existe numa versão só.** Não mencione versões anteriores nem protótipos em
código, comentários, mensagens, testes, notas ou memória: o risco de contaminar o
contexto com sintaxe e decisões que não valem mais é grande.

## Notas

Leia antes de mexer em qualquer coisa: [`notes/README.md`](notes/README.md).

- `notes/lavra.md`: **a língua como está implementada** (referência completa).
- `notes/compiler.md`: o compilador, o nativo, os testes e como crescer a língua (a lista de conferência de uma feature e os roteiros).
- `notes/morphology.md`: tabelas conferidas contra o dicionário e decisões de borda.
- `notes/tooling.md`: o suporte de editor (VS Code, servidor de linguagem, depurador, site,
  tree-sitter e os outros editores).
- `notes/roadmap.md`: o plano em curso (tooling maduro e fundações para crescer a língua).
- `notes/origin.md`: o pedido, a tese de design, as decisões que definiram a língua.
- `notes/applications.md`: as aplicações discutidas.
- `notes/poems.md`: os programas-poema e o que eles exigem da gramática.

Ao descobrir algo novo sobre a linguagem ou tomar uma decisão de design, atualize a
nota correspondente.

## Compilar e testar

```bash
cargo build                                  # compila o lavrac (o build.rs compila o runtime)
./target/debug/lavrac run samples/caixa-de-poupanca.lav
./target/debug/lavrac build samples/lavra-dos-dias.lav -o /tmp/dias && /tmp/dias
cargo test                                   # unitários, golden e paridade interpretador/nativo
LAVRA_BLESS=1 cargo test --test golden       # regrava as expectativas (revise o diff)
./target/debug/lavrac check --json arquivo.lav   # diagnósticos em JSON, com correções
./target/debug/lavrac explain L0334             # explicação de um código de erro
./target/debug/lavrac fmt --check samples        # formatador (sem --check, formata no lugar)
./target/debug/lavrac fix --check arquivo.lav     # correções seguras (sem --check, aplica)
./target/debug/lavrac dap                         # adaptador de depuração (o editor o abre)
cargo clippy --all-targets -- -D warnings && cargo fmt
tools/memcheck.sh                            # valgrind sobre toda a suíte (contêiner se o host não tem)
```

Verificar as tabelas de morfologia contra o dicionário: `tools/morphology/README.md`.

Extensão do VS Code (`editors/vscode`, ver `notes/tooling.md`):

```bash
cd editors/vscode && npm install
npm test                                     # escopos e snapshots da gramática
npm run compile                              # tipos e empacotamento do cliente
npm run test:integration                     # num VS Code de verdade (contêiner com xvfb)
cp ../../target/release/lavrac server/ && npx vsce package --target linux-x64 --out lavra.vsix
code --install-extension lavra.vsix --force
```

Gramática tree-sitter e os outros editores (`editors/tree-sitter-lavra`, `editors/helix`,
`editors/neovim`, `editors/zed`; ver `notes/tooling.md` e o README de cada pasta):

```bash
cd editors/tree-sitter-lavra && npm ci
npm run generate                             # src/ do lexicon.json e as cópias das queries
npm test                                     # corpus e realce, queries de cada editor, deriva
scripts/helix-health.sh                      # hx --health lavra e um programa aberto no Helix
scripts/neovim-check.sh                      # o plugin no Neovim headless, sobre o corpus
cd ../zed && cargo build --release --target wasm32-wasip2   # a extensão do Zed
```

Sem o Helix ou o Neovim no host, os dois roteiros rodam num `debian:trixie-slim` com o editor
da release (o do Helix pede o git, o g++ e o tmux; o do Neovim, o gcc). As queries canônicas
ficam em `queries/`, e as cópias dos editores saem do `npm run generate`, nunca à mão; à mão
só o que é de um editor (o recuo do Helix e do Zed, os pares e o esboço do Zed).

Site e playground (`site/`, ver `notes/tooling.md`): o wasm em `playground/`, depois
`cd site && npm ci && node test-wasm.mjs && node build.mjs && node check-browser.mjs`.

Operação embutida nova? O cabeçalho numa biblioteca (`src/libraries/`), a variante em
`src/builtins.rs` (com `interpret`) e a função `rt_*` em `runtime/lib.rs`; ver `notes/compiler.md`.

Palavra nova na língua? Ela entra em `src/lexicon.rs` (com papel e documentação); depois
`LAVRA_BLESS=1 cargo test --lib lexicon` regrava `editors/lexicon.json`, de onde as
gramáticas dos editores tiram as palavras (a do tree-sitter se regera com `npm run generate`
em `editors/tree-sitter-lavra`). Mudou uma regra do realce? Edite
`syntaxes/build.py` (nunca o JSON), rode `npm test`, revise e atualize os snapshots
(`npm run test:update-snapshots`), confira a renderização com `scripts/preview.mjs` e
reinstale.

## Convenções

- **Identificadores de código em inglês** (tipos, funções, variáveis, arquivos,
  pastas).
  A superfície da linguagem (palavras-chave, verbos-núcleo, tipos, mensagens de erro,
  exemplos `.lav`) é produto e continua em português.
- Prosa (comentários, docs, mensagens de erro, commits) em português com acentuação
  completa, sem travessão.
- Toda mudança na gramática precisa de programa certo e programa errado de teste, com
  a mensagem de erro esperada (`tests/programs/ok` e `tests/programs/err`).
- Mensagens de erro concordam em gênero e número com o nome de que falam.
- Todo diagnóstico tem código (`Diagnostic::new(L0334, ...)`, registro em `src/codes.rs`).
  Erro novo ganha código novo (nunca se reaproveita um) e o texto em
  `src/explain/LXXXX.md`, com exemplo errado e certo que `tests/explain.rs` confere.
- A análise não para no primeiro erro: erro novo não pode gerar cascata (o tipo de erro
  passa calado; nome de declaração quebrada continua existindo).
- Paridade entre interpretador e nativo é o critério de correção do backend: mesma
  saída byte a byte.
- Mudança no realce de sintaxe: renderize com `editors/vscode/scripts/preview.mjs` e
  confira a captura antes de dar como pronto.
