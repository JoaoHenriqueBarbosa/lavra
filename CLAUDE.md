# Lavra

Linguagem de programação de baixo nível e compilada em que cada instrução é uma
oração em português, e a gramática é verificação: artigo marca variável, modo verbal
marca o papel da oração, regência marca os argumentos, concordância é checagem
redundante. Extensão `.lav`.

## Estado

O compilador em Rust (`src/`, `runtime/`, crate `lavrac`) implementa a Lavra, "um Rust
com gramática e morfologia": interpretador e nativo (Cranelift), posse e empréstimo,
talvez e falha, registros e enumerações. Programas de exemplo e poemas em `samples/`.
Extensão do VS Code em `editors/vscode`.

**A Lavra existe numa versão só.** Não mencione versões anteriores nem protótipos em
código, comentários, mensagens, testes, notas ou memória: o risco de contaminar o
contexto com sintaxe e decisões que não valem mais é grande.

## Notas

Leia antes de mexer em qualquer coisa: [`notes/README.md`](notes/README.md).

- `notes/lavra.md`: **a língua como está implementada** (referência completa).
- `notes/compiler.md`: o compilador, o nativo e os testes.
- `notes/morphology.md`: tabelas conferidas contra o dicionário e decisões de borda.
- `notes/tooling.md`: extensão do VS Code e o caminho para o servidor de linguagem.
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

Palavra nova na língua? Ela entra em `src/lexicon.rs` (com papel e documentação); depois
`LAVRA_BLESS=1 cargo test --lib lexicon` regrava `editors/lexicon.json`, de onde a
gramática do editor tira as palavras. Mudou uma regra do realce? Edite
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
