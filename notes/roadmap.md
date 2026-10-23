# Lavra: tooling maduro e crescimento sem retrabalho

Plano aprovado em 22/10/2026. O andamento fica registrado em [journal.md](journal.md); cada item
concluído ganha a marca **(feito)** na tabela de ordem de execução.

## Contexto

O compilador (`src/`, 12 mil linhas) implementa a língua inteira do plano original: interpretador e nativo com paridade, posse, talvez e falha, registros e enumerações. A extensão do VS Code só tem realce TextMate. O pedido é duplo:

- terminar o tooling até o nível de uma linguagem séria;
- preparar a arquitetura para as features que faltam, de modo que crescer a língua não obrigue a reescrever o compilador nem o tooling.

O levantamento achou cinco travas. Elas bloqueiam o servidor de linguagem e também encarecem qualquer feature nova.

1. **Para no primeiro erro.** Toda etapa devolve `Result<T, Box<Diagnostic>>` (`src/diagnostics.rs:102`) e `compile` encadeia com `?` (`src/lib.rs:20-27`). Os avisos se perdem quando há erro.
2. **Nada liga posição a significado.** A sema descarta os escopos ao sair (`src/sema.rs:285-319`). `hir::Global` não tem span, a maioria dos `hir::Stmt` também não (`src/hir.rs:63-127`), registros e enumerações não entram no `hir::Program`. Gênero e número se perdem em `into_func`.
3. **Léxico duplicado.** As palavras da língua estão espalhadas pelo parser e pela morfologia, e repetidas à mão em `editors/vscode/syntaxes/build.py`. `RESERVED_FORMS` (`src/parser/header.rs:19-64`) é mantida à mão.
4. **Uma operação embutida nova toca nove lugares.** `Push` passa por ast, parser, sema, hir, ownership, interp, codegen e runtime.
5. **Posições sem arquivo.** `Span` não tem id de arquivo (`src/span.rs:5-10`). A coluna é contada em chars, não em UTF-16. Há um índice de linhas duplicado em `src/ownership.rs:26-43`.

**Princípio.** Tudo que o editor, os docs e as gramáticas mostram sai do compilador, e cada coisa tem uma fonte só:

- o léxico;
- o vocabulário do programa;
- as tabelas de símbolos;
- as expectativas do parser;
- o catálogo de diagnósticos.

As gramáticas externas (TextMate, tree-sitter) são rasas e geradas. O corpus (`tests/programs` e `samples/`) detecta qualquer deriva.

---

## Parte 1: o tooling

### 1.1 CLI `lavrac`

- **Subcomandos com `clap`** e `--help` em português:
  - `run`, `build` e `check`, este com `--json`;
  - `fmt` (`--check`), `lsp` e `dap`;
  - `explain <código>`, `lexicon --json` e `fix`.
- **`--version`** traz o hash do git.
- **Panic hook** com o pedido de relato: "isto é um defeito do lavrac; abra uma issue com o programa".
- **Códigos de saída mantidos:** 1 para erro ou falha, 101 para erro de execução, 2 para uso.

### 1.2 Servidor de linguagem (`lavrac lsp`)

**Base técnica**
- Stdio síncrono com `lsp-server` e `lsp-types`.
- Reanálise completa a cada mudança, já que os arquivos são pequenos. `analyze` é pura, sem estado entre requisições.
- `catch_unwind` por requisição e log em stderr.
- Codificação de posição: UTF-8 quando o cliente aceita, UTF-16 nos outros casos.
- Contrações: as duas metades têm o mesmo span (`src/lexer.rs:481,487`). Viram um token só no editor, porque o LSP proíbe sobreposição.

**Capacidades**
- **Diagnósticos**
  - Saem todos, com código e link para o catálogo.
  - Os rótulos viram `relatedInformation`.
  - "Senão" sobrando e o que não se usa levam `Unnecessary`.
- **Tokens semânticos**
  - Tipos: verbos, parâmetros, variáveis, campos, tipos e casos.
  - Modificadores padrão: `declaration`, `readonly` ("é") e `modification`.
  - Modificadores próprios: `borrowed`, `lent` e `moved`. O valor já entregue aparece apagado. **(feito: no VS Code, o `moved` sai em itálico, porque o tema não dá opacidade por escopo)**
  - As palavras da língua vêm dos papéis do léxico.
- **Hover**
  - Nomes: nome com artigo, fixo ou variável, o tipo por extenso, o modo de posse, onde foi declarado, e o estado naquele ponto ("entregue a arquivar na linha 6"). **(feito)**
  - Verbos: a regência, por `FuncSig::usage()`, e se pode falhar.
  - Palavras da língua: a linha de documentação do léxico.
- **Navegação:** definição, referências, realce de leitura e escrita, símbolos do documento e do workspace.
- **Completar guiado pelo parser** (fase 3, abaixo).
- **Signature help:** a regência com o slot ativo.
- **Inlay hints:** o tipo inferido, o modo de cada argumento e o tipo de "o resultado".
- **Code actions:** saem do `Fix` dos diagnósticos (1.6).
- **Rename com morfologia** (fase 4).
- **Formatação:** pelo formatador da 1.3.
- **Folding e selection ranges.**

**Completar guiado pelo parser.** Esta é a peça que impede que gramática nova exija mexer no servidor.
- O léxico aceita um marcador de cursor.
- Os auxiliares do parser registram o que aceitariam quando encontram o marcador: `at_word`, `eat_word`, `expect_word`, `at_punct` e `expect_punct` (`src/parser/mod.rs:88-133`).
- As consultas de classe também registram: verbo em 1ª pessoa, nome com gênero, tipo, caso de enumeração, campo e preposição da regência.
- O servidor só preenche as classes com o vocabulário e os escopos.
- Com isso, uma construção nova escrita com esses auxiliares já sai no completar.

**Testes:** `tests/lsp.rs` roda o servidor em processo (`Connection::memory()`) sobre fixtures com marcador de cursor.

### 1.3 Formatador (`lavrac fmt`)

**Regra central: nunca muda quebras de linha nem linhas em branco**, porque os poemas são programas.

Normaliza só o seguinte:
- o recuo das linhas que abrem linha lógica, pela profundidade;
- as linhas de enjambement, que mantêm o recuo relativo (os poemas usam dois níveis: `samples/catilinaria.lav:15-33`);
- tabs;
- espaço entre tokens e ao redor da pontuação;
- espaço no fim da linha e quebra final.

Nunca toca o interior de um token, como `8,5` ou `1 000 000`.

**Como funciona:** o léxico ganha um modo com trivia, em que comentário vira token.

**Invariantes, testadas no corpus:**
- é idempotente;
- a sequência de tokens, comentários incluídos, fica idêntica;
- `samples/` sai byte a byte igual.

### 1.4 Depurador (`lavrac dap`)

Roda sobre o interpretador, que é a referência. Isso realiza o "ver a frase executando" de `notes/applications.md:20-21`.

**Mudanças no `Machine` (`src/interp.rs`)**
- Os quadros passam a ser donos: `frames: Vec<Frame { func, call_span, values }>`. Hoje cada quadro é um `&mut [Value]` local a `call` (`interp.rs:84-103`).
- Um `Tracer` é chamado em `block` antes de cada oração. Disparos com o mesmo início são coalescidos, porque o `Let` escondido de "o resultado" dobraria cada oração.
- A pilha é capturada quando uma oração devolve erro.
- A saída vira um escritor que emite eventos `output`, com flush por oração.
- Pausar é a thread do interpretador bloquear num `Condvar`. O laço do DAP fica em outra thread.

**Protocolo**
- Requisições: initialize, launch, setBreakpoints por linha, threads, stackTrace, scopes (Parâmetros, Locais, Constantes) e variables.
- Em variables, registros abrem em campos, listas em "o item 1", "o item 2", e "talvez" mostra "nada" ou o valor. Os locais escondidos não aparecem.
- Execução: continue, next, stepIn, stepOut e pause.
- Também: evaluate de nomes, e exceção na "Falha".

**Entrada:** primeiro um arquivo de stdin no launch. Depois `runInTerminal`, com o depurado em processo separado ligado por socket local.

### 1.5 Extensão do VS Code

**Cliente**
- TypeScript, `vscode-languageclient` e esbuild.

**Configuração e servidor**
- Settings: `lavra.server.path`, `lavra.trace.server` e `lavra.inlayHints.*`. **(feito)**
- Servidor procurado nesta ordem: o setting, depois `lavrac` no PATH, depois o binário embutido na VSIX da plataforma.

**Comandos**
- Rodar, compilar, verificar e reiniciar o servidor.
- Botão de rodar no editor.
- Task provider com `problemMatcher` para `--> arquivo:linha:coluna`.

**Depurador e realce**
- `debuggers` do tipo `lavra`: F5 funciona sem launch.json.
- `semanticTokenScopes` para os modificadores próprios.
- Snippets dos blocos.

**Teste de integração**
- `@vscode/test-electron` sob xvfb, num contêiner.
- Confere diagnósticos, hover e completar pela API do VS Code.
- Tira uma captura de tela do editor com os tokens semânticos.

### 1.6 Diagnósticos de nível profissional

- **Código estável** num namespace único, independente da categoria: `L0001` em diante.
  - Fica num registro central (`src/codes.rs`, incluído também no runtime) com a categoria e um título.
  - Exemplo de cabeçalho: `Erro de concordância [L0103]: ...`.
  - `lavrac explain L0103` mostra um texto com um exemplo errado e um certo.
  - Um teste confere três coisas: todo código tem explicação, o exemplo errado dispara exatamente aquele código, e o certo compila.
- **`Fix { title, edits: Vec<(Span, String)>, safe }`** só com tipos que o runtime aceita.
  - Começa pelos pontos em que o compilador já calcula a forma certa: contração, concordância de artigo, verbo do sujeito, subjuntivo, ponto final, "Senão, se" e acento.
- **`check --json`**, para outros editores e para fixtures.

### 1.7 Outros editores (tree-sitter) **(feito)**

- **`editors/tree-sitter-lavra`** é raso e de propósito: orações, blocos, cabeçalhos, comentários, textos, números, palavras por papel e nomes.
  - As palavras vêm de `editors/lexicon.json`.
  - Um scanner externo em C cuida de INDENT, DEDENT, Newline, enjambement e `--`.
  - O detalhe fino fica para os tokens semânticos do servidor.
- **Teste de deriva:** todo arquivo do corpus tem de parsear sem nó ERROR.
- **Queries:** highlights, folds, indents e locals.
- **Configurações para editores:** Helix, Neovim e Zed.
- **Conferência:** `hx --health lavra` num contêiner.

### 1.8 CI, releases, documentação

- **CI no GitHub Actions:**
  - fmt, clippy e `cargo test`;
  - `npm test` e o léxico em dia;
  - `lavrac fmt --check` no corpus;
  - build wasm e valgrind.
- **Releases:**
  - binários musl para x86_64 e aarch64;
  - VSIX por plataforma com o binário embutido;
  - GitHub Release;
  - crates.io, Marketplace e Open VSX pelos secrets do repositório (ver Procedimento).
- **Arquivos na raiz:** README, LICENSE e CHANGELOG.
- **Site estático no GitHub Pages:**
  - a referência, o léxico e o catálogo de diagnósticos, gerados;
  - os poemas;
  - um playground com o interpretador em wasm32.

---

## Parte 2: crescer a língua sem retrabalho

### 2.1 Fundações (fase 0)

**F1. Tolerância a erro.** Um `Diagnostics` (sink) atravessa as etapas. O `Result` continua no controle local.

- **Léxico**
  - Os erros fatais viram diagnóstico e o léxico segue (`lexer.rs:165,253,281,292,322-332,340,350,376-439`).
  - Recuo inconsistente fica no nível mais próximo abaixo, como no Python.
- **Parser**
  - Só os quatro laços capturam: `program()` (`mod.rs:185`), `block()` (`mod.rs:207`), `prescan()` (`header.rs:69-99`) e `prescan_records()` (`data.rs:42`).
  - Regras de sincronização:
    - para no `Newline` e o consome;
    - nunca consome `Indent`, `Dedent` nem `Eof`;
    - consome `.` quando a linha lógica continua;
    - se o próximo token é `Indent`, lê o bloco como filho de `Stmt::Error`;
    - um "Se" quebrado consome a cadeia de "Senão";
    - toda recuperação consome pelo menos um token.
  - Ponto final esquecido não sincroniza: o diagnóstico vai ao sink e a oração termina num ponto virtual.
  - Cabeçalho quebrado gera uma `FuncSig { broken }` com os parâmetros lidos e a 1ª pessoa registrada. As chamadas a ela consomem a oração sem conferir a regência.
- **Sema**
  - Usa `Ty::Error` e `StmtKind::Error`.
  - Regras anticascata:
    - declaração que falha ainda declara o nome, com `Ty::Error`;
    - nenhuma mensagem cita um operando que já é `Error`;
    - oração que falha deixa um "o resultado" com `Ty::Error`;
    - os avisos "nunca muda" e "Senão nunca acontece" são suprimidos em função com erro.
  - `main` vira `Option`: só `compile` exige "Para começar".
  - Os `unreachable!` que passam a ser alcançáveis (`sema.rs:402,518,779`) viram diagnóstico.
- **Posse**
  - Registra o erro e volta a variável a `Live`, para não repetir o mesmo erro.
- **Back-ends**
  - Recebem só programa sem erros, garantido por `Analysis::has_errors()`.
- **API**
  - `lavra::analyze(text) -> Analysis { diagnostics, tokens, trivia, ast, hir, tables }`.
  - `compile` vira atalho.
  - Os diagnósticos saem ordenados por posição.
- **Critério da regravação do golden:** nenhum `.stderr` ganha um segundo erro que sumiria ao corrigir só o primeiro.

**F2. Códigos e correções.** Conforme a 1.6.
- Os cerca de 180 pontos de `Diagnostic::new` recebem código numa passada.
- `tests/golden.rs:85-98` passa a reconhecer o aviso com código.

**F3. HIR completo.**
- `hir::Stmt { kind, span }` (uns 60 braços mecânicos).
- `ast::Block.span`.
- `Global.span`.
- `Program.records` e `Program.enums` como `Vec`.
- `Local.gender`, `Local.number` e `Local.hidden`.
- `Func.sig`.

**F4. Posições.**
- `SourceMap` com base por arquivo e offsets globais. O tipo `Span` não muda.
- `LineIndex` compartilhado em `span.rs`, com colunas em chars e em UTF-16. O índice duplicado do ownership deixa de existir.
- `SourceFile` e o `render` de um arquivo ficam como estão para o runtime.

**F5. Tabelas como subproduto das passadas.** Um `Tables` é passado por `&mut` ao parser, à sema e à posse. Nunca há uma segunda resolução de nomes no servidor.

- **O que guarda**
  - `Def { kind, name, display, gender, number, ty, span, scope, mutable, mode, alias_of }`.
  - `Ref { def, span, article_span, form, use_kind }`.
    - `form`: substantivo singular ou plural, plural derivado de "Para cada", 1ª pessoa, infinitivo, subjuntivo, infinitivo pessoal, adjetivo e complemento fixo.
    - `use_kind`: leitura, mudança, entrega, empréstimo, confiança, chamada e tipo.
  - `Scope { parent, range }`.
  - `expr_types`, em que vence o span mais externo.
  - `moves` e o estado de posse em cada leitura.
  - Modo de cada argumento.
- **Onde cada coisa é registrada**
  - **No parser:**
    - nome de tipo, em `type_word` (`header.rs:532`);
    - nomes da tipagem do cabeçalho (`header.rs:411`);
    - complemento fixo (`header.rs:585-592`);
    - casos (`data.rs:566,611` e `CondKind::Variant`).
  - **Na sema:**
    - declarações e leituras, em `declare` e `lookup`;
    - campos, em `place_read`;
    - chamadas, em `FnCx::call` (`sema.rs:897`), com o `CallForm` que o parser passa a gravar em `ast::Call` nos quatro pontos de construção (`mod.rs:517,564,826,1278`).
    - "o resultado" vira um Def do tipo Result. "Se houver" fica ligado ao nome original por `alias_of`.
  - **Na posse:** `move_out` e cada leitura.
- **Avisos de não uso:** função nunca chamada, e constante, variável ou parâmetro sem uso. Saem de graça daqui.

**F6. Léxico como fonte única.**
- `src/lexicon.rs` com tabelas tipadas por papel:
  - palavras-chave;
  - verbos-núcleo com 1ª pessoa;
  - verbos do sujeito;
  - nominais;
  - substantivos de tipo (gênero, plural, bits);
  - preposições, contrações, ordinais, locuções e particípios;
  - uma linha de documentação por entrada.
- `RESERVED_FORMS` passa a ser derivada, com o motivo de cada palavra para a mensagem de erro. Se o conjunto derivado diferir do atual, isso é mudança de língua: o diff fica isolado e entra na seção "Mudanças" de `notes/lavra.md`.
- `lavrac lexicon --json` gera `editors/lexicon.json`, que é versionado.
  - O `build.py` lê o JSON e continua montando as regex.
  - Um `#[test]` confere que o JSON está em dia.

**F7. Vocabulário.**
- `Vocabulary { sigs, by_first_person, by_infinitive, records, enums, variants, field_names, ... }` sai dos campos de `Parser` (`mod.rs:39-60`). É produzido pelos prescans e consumido pelo parse dos corpos.
- É a base de bibliotecas, módulos e do completar.
- Regra registrada: nome de campo importado não entra no `field_names` do parse local, porque hoje ele muda o parse de "a soma de".

**F8. Features do Cargo.**
- `native` (Cranelift, com o `build.rs` condicionado a `CARGO_FEATURE_NATIVE`) e `cli` (clap, lsp-server, lsp-types). As duas são padrão.
- A lib compila para wasm32 com `--no-default-features`.

**F9. Robustez.** Feita primeiro, como rede de segurança.
- Cada arquivo do corpus é truncado em cada limite de oração e sofre remoções determinísticas de tokens.
- Condições conferidas:
  - nunca há panic;
  - todo span cabe no texto;
  - duas rodadas dão a mesma saída;
  - o tempo é limitado.
- Panics prováveis a corrigir: `header.rs:499` e `mod.rs:691`.

### 2.2 Pontos de extensão

Cada ponto é **provado** com um uso real e pequeno, ou fica **documentado como roteiro**.

- **Builtins em sintaxe Lavra (provado).**
  - Cada operação é declarada por um cabeçalho Lavra, parseado pelo próprio `header()`.
  - Regência, posse e tipos vêm de graça. Hover, completar e signature help também, sem tocar o servidor.
  - `ExprKind::Call { callee: User | Builtin }`.
  - A implementação é dupla, no interpretador e no runtime, e a paridade testa as duas.
  - Uma operação nova toca 4 ou 5 lugares em vez de 9.
  - Os verbos-núcleo, que têm sintaxe própria em `clause()`, continuam fixos.
- **Bibliotecas importadas (provado).**
  - "Uso a biblioteca de textos." é processado antes do `prescan()`.
  - Verbo de biblioteca não reserva palavra em programa que não importa a biblioteca.
  - Colisão com verbo do usuário é erro com dica.
  - A primeira biblioteca, `textos`, traz poucos verbos de desenho óbvio (dividir, juntar, conter, aparar). Cada um tem programa certo e errado.
  - O teste de aceite: o servidor de linguagem atende a biblioteca sem nenhuma mudança no seu código.
- **Módulos do usuário (roteiro).** `SourceMap` mais `Vocabulary` mais ids globais. A sintaxe fica para quando for desenhada. **(roteiro em `notes/compiler.md`)**
- **Genéricos e traits (roteiro).** `Ty::Param` e monomorfização entre a sema e os back-ends. Nenhum back-end vê variável de tipo. Interning de tipos entra nessa hora. **(roteiro em `notes/compiler.md`)**
- **Referências guardadas (roteiro).** `LoanKind::Binding` lexical, ao lado de `Loop` e `Match` (`ownership.rs:109-113`). Fica documentado que é o modelo lexical, não o NLL. **(roteiro em `notes/compiler.md`)**
- **Tipos novos no runtime (roteiro).** Um `KIND_*` novo no `Desc` (`runtime/lib.rs:37-48`), escrito num lugar só do codegen. **(roteiro em `notes/compiler.md`)**

### 2.3 Processo

- **Checklist por feature em `notes/compiler.md`:** **(feito)**
  - léxico, parser, sema, HIR, posse, interpretador, codegen e runtime;
  - programa certo e errado;
  - golden e paridade;
  - léxico regerado;
  - nota da língua.
  - A gramática do editor e o servidor ficam fora da lista, porque se regeneram.
- **O corpus é a suíte de compatibilidade.**
  - Mudança incompatível é aceita antes da 1.0 se `tests/programs`, `samples/` e "Mudanças" mudarem no mesmo commit.
  - Migração mecânica vira `Fix`, e `lavrac fix` aplica.
- **A paridade continua sendo o critério do back-end.**

### 2.4 Catálogo de features previstas

| Feature | Depende de | Impacto |
|---|---|---|
| "talvez" no cabeçalho, no parâmetro e no retorno (`notes/lavra.md`, "Ainda falta") | `type_word` do cabeçalho, sema | aditiva |
| Interpretador sem cópia ao passar coleção emprestada ou confiada (`notes/compiler.md`, custo de cópia) | quadros donos do depurador (fase 5) | interna, sem mudança de saída |
| Texto→número, caractere→inteiro | `como` | aditiva |
| Texto, matemática, arquivos, tempo | builtins e bibliotecas | aditiva |
| Numerais por extenso | léxico | aditiva; reserva palavras |
| `Pattern::Literal`, `CondKind::Empty/Contains` (já na AST) | sema, back-ends | aditiva |
| Mapas e conjuntos | `KIND_*`, builtins | aditiva |
| Registro recursivo via lista (`sema.rs:195`) | descritores | aditiva |
| Módulos | F4, F7 | aditiva |
| Referências guardadas | loans lexicais | aditiva; regras novas |
| Genéricos, traits | monomorfização | aditiva |
| Verbos de primeira classe | HIR, ponteiro de função | aditiva |
| Nativo em macOS e Windows | linha de link por SO | aditiva |

---

## Ordem de execução

Cada item termina com a suíte verde, um commit e um push.

| # | Item | Pronto quando |
|---|---|---|
| 0.1 | CI mínimo; este plano vira `notes/roadmap.md` **(feito)** | Actions verde |
| 0.2 | F9, robustez **(feito)** | o teste passa sobre o corpus mutilado |
| 0.3 | F6, léxico **(feito)** | `build.py` lê o JSON; snapshots iguais |
| 0.4 | F7, vocabulário **(feito)** | suíte verde, sem mudança de saída |
| 0.5 | F1 e F2, tolerância e códigos **(feito)** | `check` lista todos os erros; golden revisado; teste do explain |
| 0.6 | F3 e F4, HIR e posições **(feito)** | suíte verde |
| 0.7 | F5, tabelas e não uso **(feito)** | testes de tabela sobre fixtures |
| 0.8 | F8, CLI própria em português, versão, panic hook **(feito)** | `cargo check --lib --no-default-features --target wasm32-unknown-unknown` |
| 1 | Servidor (diagnósticos, tokens, hover, navegação, símbolos) e cliente **(feito)** | teste de integração e captura no VS Code |
| 2 | Formatador **(feito)** | `fmt --check` no corpus sem alterar os poemas |
| 3 | Completar pelo parser, signature help, inlay, code actions, folding **(feito)** | fixtures em `tests/lsp` |
| 4 | Rename com morfologia e `lavrac fix` **(feito)** | rename de verbo num sample, com a suíte verde |
| 5 | Depurador e F5 **(feito)** | sessão roteirizada; captura parado num breakpoint |
| 6 | Builtins e `Uso a biblioteca de textos.` **(feito)** | biblioteca no servidor sem mudar o servidor |
| 7 | Releases, README, CHANGELOG, publicação **(feito; crates.io e lojas esperam os tokens)** | instalação limpa num contêiner |
| 8 | tree-sitter e configs de editores **(feito)** | corpus sem ERROR; `hx --health` |
| 9 | Site e playground **(feito)** | um poema rodando no navegador |

## Procedimento

- **Commits e push.** Commit em português, sem assinatura de IA, e push para `main` ao fim de cada item. O CI roda em cada push.
  - O user também commita de outro computador. Antes de cada push vem um `git pull --rebase`. Conflito em arquivo dele se resolve preservando o conteúdo dele, e nunca há push forçado.
- **Diário.** `notes/journal.md` recebe uma entrada ao fim de cada fase (e no meio da fase 0, que é longa). Cada entrada traz data, tempo decorrido, o que foi feito, os números e as decisões a revisar.
- **Notas.**
  - Atualizadas no item que as afeta: `tooling.md`, `compiler.md`, `lavra.md` (com "Mudanças"), `README.md` e o `CLAUDE.md`.
  - A memória do projeto é atualizada no fim.
- **Agentes.** Frentes independentes podem ir para agentes em paralelo, com briefing fechado: os textos do explain, o cliente TypeScript, o tree-sitter e o site. Eu integro e verifico.
- **Toolchains.** O que não está no host roda em Docker: wasm32, xvfb com o VS Code de teste, Helix. Python usa uv.
- **Publicação externa.**
  - A pipeline fica pronta e o GitHub Release sai.
  - crates.io, Marketplace e Open VSX dependem de tokens. Se eles não estiverem disponíveis (o navegador automatizado não está nesta sessão), isso fica registrado como bloqueio, sem rebaixar o resto.

## Fora do escopo agora

- **Gestão de pacotes:** ainda não há módulos.
- **`lavrac test`:** a língua não tem asserção. O golden com `.stdout` já é o framework de teste.
- **REPL:** exige análise incremental e conflita com a pré-varredura do programa inteiro.
- **DWARF no nativo:** o runtime já leva spans nas falhas.
- **Gerador de docs de programa:** fica para quando houver bibliotecas de usuário.

## Decisões tomadas por padrão (entram no diário para revisão)

1. O código aparece no terminal: `Erro de concordância [L0103]: ...`.
2. Colisão entre verbo de biblioteca e verbo do usuário é erro.
3. O formatador nunca mexe em quebras de linha.
4. O rename recebe o nome novo com artigo. Sem artigo, o gênero é inferido pela terminação, e o rename recusa quando a inferência falha.
5. O depurador sai primeiro com stdin por arquivo. O terminal integrado vem na mesma fase.

## Verificação

- **A cada item:** `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `npm test` e `tools/memcheck.sh`.
- **Fase 0:**
  - diff dos `.stderr` revisado um a um, sem cascatas;
  - o `RESERVED_FORMS` derivado é comparado com o atual;
  - os snapshots do TextMate continuam iguais.
- **Servidor:** fixtures em processo e teste de integração do VS Code sob xvfb, com captura de tela.
- **Formatador:** idempotência e preservação de tokens no corpus.
- **Depurador:** sessões roteirizadas e captura de tela.
- **Releases:** instalação do zero num contêiner limpo, rodando um sample.
- **tree-sitter:** corpus sem ERROR e `hx --health lavra`.
