# Suporte de editor

## VS Code (`editors/vscode`)

Extensão `joaohenriquebarbosa.lavra`, instalada no VS Code do WSL em 2026-10-22 com
`code --install-extension lavra.vsix`.

- **Gramática TextMate** `source.lavra`, gerada por `syntaxes/build.py` a partir do
  léxico que o compilador exporta (`editors/lexicon.json`, de `lavrac lexicon --json`):
  as palavras vêm do léxico e o `build.py` só monta as regras; o JSON não se edita à mão.
  Palavra nova na língua entra em `src/lexicon.rs`, o JSON se regera
  (`LAVRA_BLESS=1 cargo test --lib lexicon`) e a gramática também (`npm test`).
  Escopos padrão para os temas pintarem sem configuração: `storage.type.declaration` (é/são), `storage.modifier.mutable`
  (começa), `keyword.other.mutation` (vira, cresce, ganha, perde, diminui),
  `keyword.control.*`, `entity.name.function` (cabeçalho, predicado, chamada em 1ª
  pessoa, verbo depois de "o resultado de", adjetivo depois de "for"),
  `variable.parameter`, `variable.other` (nome com artigo), `variable.language.result`
  (o resultado), `storage.modifier.borrowed/lent/maybe` (emprestado, confiado, talvez),
  `storage.type.primitive`, `entity.name.type` (registros, enumerações, casos),
  `support.function.nominal` (a soma de ...), `keyword.operator.*`, literais e comentários.
  O artigo leva o escopo da palavra que determina ("o tamanho", "o resultado", "o item");
  a contração leva o de preposição ("seguido do tamanho").
- Fronteira de palavra própria (`\b` do Oniguruma não serve para "é"); tudo sem
  diferenciar maiúsculas, porque a primeira palavra da oração vem maiúscula.
- O TextMate escolhe o casamento que começa antes na linha; por isso a regra de chamada
  do usuário (ancorada na margem) exclui explicitamente as palavras da língua e as
  continuações de linha.
- **Limite do léxico**: o verbo de predicado em `Se o saldo exceder o teto` e o
  complemento fixo (`os dias` em "contar os dias até") não se distinguem pela forma;
  vão precisar dos tokens semânticos do servidor de linguagem.
- **Configuração**: comentário `--`, pares, recuo depois de `:`, `Senão` desrecua, dobra
  por indentação (`offSide`), palavra com acento e hífen.

### Testes

- `npm test`: asserções de escopo (`tests/syntax/*.lav`, formato do
  `vscode-tmgrammar-test`, geradas por `scripts/make_syntax_tests.py`) e snapshots de
  todos os `samples/` (`tests/snap`, links para os samples).
- `node scripts/tokens.mjs arquivo.lav`: escopos de cada token, com o motor do VS Code.
- `node scripts/preview.mjs saída.html arquivos...`: renderiza com shiki nos temas
  Dark+ e Light+; a captura sai com `chromium --headless=new --screenshot`.

## Cliente do VS Code

`editors/vscode/src/extension.ts`, em TypeScript com `vscode-languageclient` 11,
empacotado pelo esbuild em `dist/extension.js` (o `tsc` só confere os tipos). Procura o
`lavrac` no setting `lavra.server.path`, depois em `server/lavrac` dentro da extensão (a VSIX
por plataforma leva o binário), depois no PATH. Comandos "Lavra: Rodar o arquivo",
"Compilar para nativo", "Verificar" e "Reiniciar o servidor de linguagem"; botão de rodar no
título do editor; tarefas do tipo `lavra` com os casadores de problema `$lavrac` (erros) e
`$lavrac-aviso` (avisos), separados porque "Erro" e "Aviso" não são as severidades em
inglês que o VS Code entende; modificadores semânticos `borrowed`, `lent` e `moved`; o `moved`
sai em itálico por uma regra em `configurationDefaults` (`editor.semanticTokenColorCustomizations`,
`"*.moved:lavra": {"italic": true}`), que só liga o itálico e deixa a cor com o tema. Um
mapeamento para o escopo `markup.italic` trocaria a cor: os temas padrão pintam esse escopo
(roxo no Dark+, verde no Abyss). Snippets dos blocos.

Teste de integração (`npm run test:integration`): um VS Code de verdade, baixado pelo
`@vscode/test-electron`, abre um programa com erro e confere pela API do editor os
diagnósticos (com código e link), o hover, os tokens semânticos, o completar e a definição.
Aqui roda num contêiner `node:22-trixie` com xvfb (como root, com `--no-sandbox`; a glibc
do contêiner tem de ser a do host, onde o `lavrac` foi compilado) e, com
`LAVRA_SHOT=caminho.png`, tira três capturas da tela: o hover, uma oração nova com o
completar aberto e os inlay hints, e a ajuda de assinatura (`-completar.png`,
`-assinatura.png`); no CI, com o `xvfb-run` do runner. Os arquivos que o contêiner cria ficam
com o dono do mapeamento de usuários do Docker Desktop: apague `out/` com um contêiner.

## Formatador (`lavrac fmt`)

`src/format.rs`, conservador por desenho porque os poemas são programas: nunca muda quebra
de linha nem linha em branco, nem o interior de um token. Refaz o recuo das linhas que
abrem linha lógica (quatro espaços por bloco aberto, pelo papel de cada linha física que o
léxico informa em `lexer::layout`), leva as linhas de continuação junto mantendo a
distância que tinham, deixa um espaço entre tokens (nenhum antes de `.`, `,`, `:`, `;` e `)`
nem depois de `(`; dentro das locuções também), tira espaço do fim da linha e garante a
quebra final. Dois números vizinhos guardam o espaço original, porque um só os juntaria
num número com milhar. Texto com erro léxico não se formata.

`lavrac fmt arquivos ou pastas` formata no lugar; `--check` só diz o que mudaria (o CI
confere samples e programas certos). O servidor de linguagem responde
`textDocument/formatting`. Testes (`tests/format.rs`): o corpus já está na forma; versões
estragadas (recuo com tab, espaços a mais, espaço no fim) voltam byte a byte, com os mesmos
tokens e comentários; formatar de novo não muda nada.

## Servidor de linguagem (`lavrac lsp`)

`src/lsp/` (feature `lsp`, padrão), síncrono sobre stdio com `lsp-server` e `lsp-types`.
Cada mudança reanalisa o documento inteiro com `lavra::analyze`, que não para no primeiro
erro; cada requisição roda isolada (`catch_unwind`), e um defeito vira erro só dela.

- **Posições**: UTF-8 quando o cliente aceita, UTF-16 nos outros (o VS Code), pelo
  `LineIndex` de `span.rs`.
- **Diagnósticos**: todos, com o código, o link para a explicação, a dica e as notas na
  mensagem, os rótulos como informação relacionada, `Unnecessary` nos avisos de não uso e
  no "Senão" que nunca acontece, e as correções em `data` (para as ações rápidas).
- **Tokens semânticos**: das tabelas da análise (funções, também nos predicados e depois de
  "o resultado de"; parâmetros com `borrowed` e `lent`; variáveis, `readonly` para o que "é";
  campos, tipos e casos; `declaration` e `modification`; `moved` no nome que entrega o valor)
  e do vocabulário (tipos e casos citados). As palavras da língua ficam com a gramática
  TextMate. A contração conta um token.
- **Hover**: nome com artigo, fixo ou variável, o tipo por extenso, o modo de posse e a linha
  da declaração; verbo com a regência, o retorno e se pode falhar; palavra da língua com a
  documentação do léxico. O estado de posse naquele ponto vem da própria análise de posse
  (`Tables.ownership`): onde o valor é entregue ("As cartas são entregues a \"arquivar\"
  aqui: daqui em diante, não se usam mais.") e onde o nome que o entregou ganha um valor
  novo ("foram entregues a \"arquivar\" na linha 8; aqui ganham um valor novo").
- **Navegação**: definição, referências (com leitura e escrita), destaque, símbolos do
  documento (funções com a regência, registros com campos, enumerações com casos,
  constantes) e do workspace (documentos abertos e `.lav` das pastas). A definição de um
  verbo de biblioteca fica no texto da biblioteca, no endereço `lavra-library:/textos.lav`,
  e o servidor só a devolve ao cliente que avisa, nas opções de inicialização
  (`libraryDocuments`), que sabe abrir esse endereço; o texto vem do pedido `lavra/library`.
  O cliente do VS Code abre o texto só para leitura e não o manda ao servidor (o seletor de
  documentos fica com `file` e `untitled`); nos outros editores, o hover diz de que
  biblioteca o verbo vem.
- **Completar guiado pelo parser**: o servidor põe a palavra `lavracursorx` no lugar da
  palavra em que está o cursor e roda o parser com ela marcada; os auxiliares do parser
  (`at_word`, `eat_word`, `expect_word`), o começo de oração, o nome depois do artigo, o
  objeto de uma chamada e a posição de tipo registram o que aceitariam ali
  (`parser::expectations`). O servidor só preenche essas classes: verbos da língua e do
  programa (o do programa vira snippet com a regência), o dado como sujeito, nomes visíveis
  do gênero e do número do artigo, campos, operações por extenso, posições, tipos. Construção
  nova escrita com os auxiliares já aparece no completar, sem mexer no servidor.
- **Ajuda de assinatura**: a regência do verbo da oração, com o parâmetro ativo pela última
  preposição da regência já escrita.
- **Inlay hints**: o tipo das declarações sem aposto (", um inteiro,"), "confiado:" e
  "emprestado:" antes dos argumentos, e o tipo de "o resultado".
- **Ações rápidas**: as correções que vêm nos diagnósticos (contração, concordância, ponto
  final...), a segura como preferida.
- **Dobra e seleção**: pelo papel das linhas que o léxico informa (blocos recuados,
  sequências de comentários; palavra, oração, blocos em volta, texto).
- **Rename com morfologia** (`src/rename.rs`, `prepareRename` e `rename`): o editor propõe
  o nome com o artigo ("o saldo", "uma conta") ou o infinitivo ("render"). Ver a seção
  abaixo.
- **Corrigir tudo** (`source.fixAll`): as correções seguras do documento inteiro numa ação
  só, as mesmas do `lavrac fix`; vale como ação ao salvar
  (`"editor.codeActionsOnSave": {"source.fixAll": "explicit"}`).
- **Bibliotecas**: os verbos de uma biblioteca trazida ("Uso a biblioteca de textos.") saem
  do vocabulário e das tabelas como os do programa, então completar, hover, assinatura,
  tokens semânticos e referências valem sem nada no servidor que saiba de bibliotecas. A
  única regra do servidor é genérica: o que tem posição fora do documento (a definição na
  biblioteca) não vira lugar nele; o hover diz "Vem da biblioteca de textos." no lugar da
  linha. `tests/lsp.rs` tem o aceite.
- **Testes**: `tests/lsp.rs` roda o servidor em processo (`Connection::memory()`).

## Rename com morfologia

`src/rename.rs`. Cada uso registrado nas tabelas guarda a forma em que foi escrito
(`tables::Form`), e o rename reescreve cada um na mesma forma:

- **Função**: 1ª pessoa, infinitivo, futuro do subjuntivo e infinitivo pessoal (no número
  de cada uso: "excederem"), o adjetivo do predicado concordando com o sujeito de cada uso,
  e a 1ª pessoa declarada no cabeçalho ("Para trovejar (trovejo)"): troca, some quando o
  verbo novo tem a regular, ou entra quando ele pede ("chover (chovo)").
- **Nome, tipo ou caso**: a palavra (no plural onde o tipo aparece no plural), e o artigo
  ou a contração logo antes dela quando o gênero muda ("do saldo" vira "da reserva",
  "numa conta" vira "num cofre"), com a maiúscula do começo da oração. O nome novo vem com
  o artigo; sem ele, o gênero sai da terminação ("-o", "-a", "-ção", "-dade", "-agem"), e o
  rename recusa quando a terminação não decide ("valor", "problema"). O número fica, porque
  nome no plural é coleção. Símbolo troca por símbolo.
- **O mesmo nome em outra declaração**: o "Se houver apelido" e os campos que um braço de
  "Conforme" expõe vão junto (são definições no mesmo trecho, ou ligadas pelo uso). "Para
  cada nota" cita "as notas" pelo singular: renomear a coleção leva o item ao singular do
  nome novo; renomear só o item deixa a coleção dita ("Para cada avaliação das notas"). O
  item escrito que é o singular da coleção ("Para cada herdeiro dos herdeiros") também vai
  junto com ela. O item tem o gênero da coleção, então trocar o gênero só dele é recusado.
- **O que concorda sem citar o nome** ("As compras começam vazias", "Se a luz for acesa",
  "ambos" no cabeçalho, "é uma destas", o particípio da tipagem) se ajusta com as correções
  seguras do próprio compilador, aplicadas só aos erros de concordância que o rename fez
  aparecer e nunca ao que ele escreveu. O ordinal que tinha o gênero da coleção ("a primeira
  das notas") vai junto, porque a língua não o confere. O adjetivo do cabeçalho de um
  predicado acompanha o sujeito ("Para a peça ser quadrada").
- **Verificação**: o texto renomeado é analisado de novo. O rename é recusado, com a
  mensagem do compilador, se aparecer erro que não havia, e é recusado se algum uso passar a
  apontar para outro nome (o nome novo esconderia um que o programa usa depois).

As edições saem pequenas, sobre o texto que o editor tem (`fix::Edited` acumula as voltas
sobre o original). `tests/rename.rs` cobre cada forma e cada recusa, renomeia cada nome de
cada programa do corpus (mais de 300) exigindo que compile e dê a mesma saída, e renomeia
programas quebrados e cortados sem entrar em pânico.

## Correções (`lavrac fix`)

`src/fix.rs`. As correções seguras dos diagnósticos (contração, concordância, ponto final,
"Senão, se", comparação no plural...) se aplicam em voltas, porque uma revela a outra (o
ponto final que faltava esconde a concordância da linha de baixo), até não sobrar
nenhuma. A correção que troca o começo de uma oração mantém a maiúscula (`keep_case` em
`analyze`). `lavrac fix arquivos ou pastas` grava e diz quantas aplicou e se ainda há
erro; `--check` só diz quais arquivos mudariam. O mesmo motor serve ao "corrigir tudo" do
editor e às concordâncias do rename. `tests/fix.rs`: idempotência sobre os programas
errados do corpus, e o erro corrigido nunca volta (o que aparece depois é erro que estava
escondido, com outro código).

## Depurador (`lavrac dap`)

Roda sobre o interpretador, que é a referência da língua: ver a frase executando.

- **Núcleo** (`src/debug.rs`, na biblioteca, sem feature): o `Debugger` é o acompanhante
  do interpretador. Recebe `Command`s por um canal e manda `Event`s por um `Sink`. Parado
  num ponto, quem responde às consultas (pilha, escopos, variáveis, avaliar) é a própria
  thread do programa, com a pilha à vista: não há estado dividido entre threads. Serve
  também a um playground.
- **Pontos de parada** por linha. O pedido numa linha sem oração pega na próxima em que
  começa uma (`statement_lines`, `place_breakpoint`). Parar é chegar à linha vindo de
  outra; as orações seguintes da mesma linha e as voltas de um laço escrito numa linha só
  não param de novo.
- **Passos**: continuar, próxima linha (por cima das chamadas), entrar, sair, pausar (um
  laço sem fim para na próxima oração). Parar na entrada (`stopOnEntry`).
- **O que se vê**: a pilha ("Para render", "Para começar", com a linha de cada chamada); os
  escopos Parâmetros, Locais e Constantes; só os locais que valem naquele ponto (pelas
  tabelas da análise), sem os escondidos e sem os que ainda não ganharam valor; lista abre
  em "o item 1", "o item 2"; registro e caso abrem nos campos, com o artigo; "talvez" mostra
  "nada" ou o valor; o texto vem entre aspas. Avaliar aceita um nome com ou sem artigo
  ("o saldo", "saldo"), e é o que o hover do editor usa.
- **Falhas**: os erros de execução param onde aconteceram, com a pilha inteira (filtro
  "Erros de execução", ligado); com o filtro "Falhas declaradas", para antes de cada
  "Desisto". A falha que encerra o programa vai para o console como o `lavrac run` a mostra,
  e o código de saída é o mesmo (101 no erro de execução, 1 na falha declarada).
- **Adaptador** (`src/dap.rs`, feature `dap`): o protocolo com o editor pela entrada e
  saída padrão. O programa roda no mesmo processo, numa thread com pilha grande, a saída
  vai para o console de depuração e a entrada vem do arquivo `"stdin"`. Com
  `"console": "integratedTerminal"` (o padrão quando o programa tem "Leio" e não há
  `"stdin"`), o editor abre um terminal com `lavrac debuggee --port N arquivo.lav`, que liga
  de volta por um socket local: o programa lê e escreve no terminal, e o depurador conversa
  pelo socket, uma mensagem JSON por linha. Programa com erro não começa: os erros vão para
  o console de depuração.
- **No VS Code**: o tipo `lavra`, F5 sem launch.json (depura o arquivo aberto, salvo antes),
  o botão "Depurar o arquivo" no título do editor, e o mesmo `lavrac` do servidor de
  linguagem.
- **Testes**: `tests/debug.rs` (o núcleo por canais: ponto, pilha, variáveis, passos, falha,
  pausa, entrada), `tests/dap.rs` (o binário pelo protocolo, no console e no terminal, com o
  teste fazendo o papel do editor no `runInTerminal`) e a integração do VS Code, que para
  num ponto, confere pilha e variáveis pelo adaptador e tira a captura parada
  (`-depurador.png`). Com `LAVRA_DAP_LOG=arquivo`, a integração grava o tráfego entre o VS
  Code e o adaptador. O VS Code só abre o primeiro escopo sozinho se a vista de depuração
  já está à mostra quando o programa para; por isso o teste a abre antes de começar.

## Site e playground

O site (`site/`, publicado pelo GitHub Pages em `.github/workflows/pages.yml`) sai inteiro do
repositório: `site/build.mjs` gera a apresentação, a referência (`notes/lavra.md`), o catálogo
de erros (`src/codes.rs` e `src/explain`), o léxico (`editors/lexicon.json`), as bibliotecas
(`src/libraries`), uma página por poema (`samples/`, com a entrada e a saída) e o playground.
O código da Lavra sai realçado pela gramática TextMate da extensão (shiki, temas Light+ e
Dark+), e o site segue o tema claro ou escuro do sistema. O fundo dos blocos é o do site: os
trechos realçados só levam a cor do texto.

O playground roda o próprio compilador no navegador: o crate `playground/` (fora do pacote do
`lavrac`, com o `[workspace]` próprio) compila a análise e o interpretador para wasm32, e o
`wasm-bindgen` gera o pacote. A página usa o CodeMirror com a língua tirada do léxico
(`site/src/lavra-language.js`) e os diagnósticos do compilador ao digitar; rodar acontece num
Web Worker (`site/src/worker.js`), com a saída saindo aos pedaços e o botão de parar
encerrando o worker, e o link de compartilhar leva o código no endereço. A pilha do motor
JavaScript (cerca de 1 MiB no V8) acaba por volta de 1 500 chamadas aninhadas da Lavra, então o
playground limita a 1 000 (`Machine::limit_depth`), e a falha é a da língua ("Estouro de pilha").

```bash
cd playground && CARGO_TARGET_DIR=../target/playground cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir ../target/playground/web ../target/playground/wasm32-unknown-unknown/release/lavra_playground.wasm
wasm-bindgen --target nodejs --out-dir ../target/playground/node ../target/playground/wasm32-unknown-unknown/release/lavra_playground.wasm
cd ../site && npm ci
node test-wasm.mjs        # o interpretador em wasm contra o corpus, no Node
node build.mjs            # gera site/dist
node check-browser.mjs    # cada poema roda no playground do Chromium e dá a saída esperada
```

Aqui o wasm se compila no contêiner `rust:1` (o alvo wasm32 e o `wasm-bindgen-cli` da mesma
versão do crate, instalado em `target/tools`), chamado de dentro de `playground/` para valer a
configuração dele, e com `--locked` para o contêiner não reescrever o `Cargo.lock`.

## Próximos passos

A gramática tree-sitter com as configurações de outros editores: `notes/roadmap.md`, fase 8.
