# Lavra

Linguagem de programação de baixo nível e compilada em que cada instrução é uma
oração em português, e a gramática é verificação: artigo marca variável, modo verbal
marca o papel da oração, regência marca os argumentos, concordância é checagem
redundante. Extensão `.lav`.

## Estado

- O compilador em Rust (`src/`, `runtime/`, crate `lavrac`) implementa a língua nova,
  "um Rust com gramática e morfologia": interpretador e nativo (Cranelift), posse e
  empréstimo, talvez e falha, registros e enumerações. Plano aprovado em
  `~/.claude/plans/bright-mixing-hoare.md`.
- `draft/` é o protótipo feito no claude.ai em 2026-10-21 (JS, x86-64, runtime C, editor
  HTML, poemas). É referência somente leitura: não edite, não gere artefatos dentro dele,
  e não cite o draft em código nem em mensagens da língua (para quem usa, só existe uma
  Lavra).
- Na raiz de `draft/`, `lavra.js` e `lavra.html` são mais novos que os de
  `draft/lavra/` (patch de contração no cabeçalho, necessário para
  `catilinaria.lav`).

## Notas

Leia antes de mexer em qualquer coisa: [`notes/README.md`](notes/README.md).

- `notes/lavra.md`: **a língua como está implementada** (referência completa).
- `notes/compiler.md`: o compilador, o nativo e os testes.
- `notes/morphology.md`: tabelas conferidas contra o dicionário e decisões de borda.
- `notes/tooling.md`: extensão do VS Code e o caminho para o servidor de linguagem.
- `notes/origin.md`: o pedido, a tese de design, a ordem dos fatos.
- `notes/language.md`: a linguagem como estava no protótipo.
- `notes/draft-architecture.md`: como o draft é construído.
- `notes/draft-status.md`: o que foi verificado, bugs e limitações.
- `notes/applications.md`: as aplicações discutidas e o roadmap.
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
cargo clippy --all-targets -- -D warnings && cargo fmt
tools/memcheck.sh                            # valgrind em contêiner sobre toda a suíte
```

Verificar as tabelas de morfologia contra o dicionário: `tools/morphology/README.md`.

Extensão do VS Code (`editors/vscode`, ver `notes/tooling.md`):

```bash
cd editors/vscode && npm install
npm test                                     # escopos e snapshots da gramática
npm run package && code --install-extension lavra.vsix --force
```

Mudou a gramática? Edite `syntaxes/build.py` (nunca o JSON), rode `npm test`, revise e
atualize os snapshots (`npm run test:update-snapshots`), confira a renderização com
`scripts/preview.mjs` e reinstale.

## Rodar o draft

Node está no host (`~/.local/bin/node`) e gcc também. `lavrac.js -s/-c` escreve `.s` e
executável ao lado do `.lav`, então rode numa cópia:

```bash
rm -rf /tmp/lavra-run && mkdir -p /tmp/lavra-run
cp -r draft/lavra/* /tmp/lavra-run/ && cp draft/lavra.js /tmp/lavra-run/ && cp draft/*.lav /tmp/lavra-run/exemplos/
cd /tmp/lavra-run
node lavrac.js exemplos/script.lav </dev/null      # interpreta (sempre dar stdin, ele lê até EOF)
node lavrac.js -c exemplos/lista2.lav && ./exemplos/lista2
```

O editor é `draft/lavra.html` (abre direto no navegador, sem servidor).

## Convenções para a reescrita

- **Identificadores de código em inglês** (classes, funções, variáveis, arquivos,
  pastas), mesmo que o draft use português (`Analisador`, `Maquina`, `falha`).
  A superfície da linguagem (palavras-chave, verbos-núcleo, tipos, mensagens de erro,
  exemplos `.lav`) é produto e continua em português.
- Prosa (comentários, docs, mensagens de erro, commits) em português com acentuação
  completa, sem travessão.
- Toda mudança na gramática precisa de programa certo e programa errado de teste, com
  a mensagem de erro esperada (`tests/programs/ok` e `tests/programs/err`).
- Mensagens de erro concordam em gênero e número com o nome de que falam.
- Paridade entre interpretador e nativo é o critério de correção do backend: mesma
  saída byte a byte.
- Mudança visual no editor: screenshot pelo chrome-devtools antes de dar como pronto.
