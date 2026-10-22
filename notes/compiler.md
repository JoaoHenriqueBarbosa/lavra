# O compilador (`lavrac`)

Crate único em Rust 2024 (`Cargo.toml`): biblioteca `lavra` e binário `lavrac`. A feature
`native` (padrão) traz o Cranelift e o runtime; `lsp` e `dap` (padrão) trazem o servidor de
linguagem e o adaptador de depuração; sem elas, a biblioteca (análise, interpretador e o
núcleo do depurador) compila para wasm32 (`cargo check --lib --no-default-features --target
wasm32-unknown-unknown`, conferido no CI), para o playground. A linha de comando é própria e
em português (o `clap` daria ajuda e erros de uso em inglês); `lavrac --version` mostra o
commit, e um pânico do compilador avisa que é defeito do compilador e pede o relato.
Dependências: o Cranelift 0.137 (`codegen`, `frontend`, `module`, `object`, `native`) e o
`serde_json` (exportação do léxico).

```
lavrac run    arquivo.lav          interpreta
lavrac dap                         o adaptador de depuração (Debug Adapter Protocol, stdio)
lavrac build  arquivo.lav [-o x]   executável nativo (padrão: o nome sem .lav)
lavrac check  arquivo.lav          só confere
lavrac check --json arquivo.lav    os diagnósticos em JSON (para editores e ferramentas)
lavrac explain L0334               explica um erro, com exemplo errado e certo
lavrac lexicon --json              exporta o léxico
```

Os diagnósticos levam correções (`Fix`) onde o compilador sabe a forma certa
(contração, concordância, subjuntivo da condição, ponto final, vírgula de "Senão, se"):
o trecho e o texto novo, marcados como seguros. O terminal mostra só a dica em prosa; o
JSON e o servidor de linguagem levam as correções. As explicações ficam em
`src/explain/LXXXX.md` (o `build.rs` embute todas), e `tests/explain.rs` confere que todo
código tem a sua, com o título do registro, e que o exemplo errado dá o código e o certo
não dá nada (os de execução rodam no interpretador).

## Pipeline

1. `lexer.rs`: tokens com spans, INDENT/DEDENT, enjambement, contrações desfeitas
   (cada metade guarda a forma escrita), locuções ("maior que", "igual a", "pelo menos").
2. `parser/` (descida recursiva): `mod.rs` (períodos, orações, condições, expressões),
   `header.rs` (pré-varredura e cabeçalhos de função; chamadas guiadas pela regência),
   `data.rs` (registros, enumerações, lugares, coleções, operações por extenso).
   Confere a gramática local: concordância verbal, contração, regência, ordinais.
   As pré-varreduras (registros e enumerações, depois os cabeçalhos "Para ...") produzem
   o `ast::Vocabulary` (verbos pela 1ª pessoa e pelo infinitivo, registros, enumerações,
   casos, nomes de campo), que a leitura dos corpos consulta e que vai junto no
   `ast::Program`; é por ele que uma biblioteca acrescenta nomes e o servidor de
   linguagem completa.
3. `sema.rs`: escopos, concordância nominal com a declaração, tipos com inferência
   (variáveis de tipo unificadas e resolvidas no fim de cada função), mutabilidade,
   exaustividade do "Conforme". Baixa para `hir.rs`.
4. `ownership.rs`: posse e empréstimo, por fluxo sobre a HIR (Se, laços com duas
   passadas, Paro/Pulo).
5. `interp.rs`: interpretador da HIR, referência de semântica. Parâmetro confiado por
   cópia na ida e na volta (equivale à referência, porque o empréstimo é exclusivo). A
   máquina é dona da pilha (`Frame`: a função, onde foi chamada, os valores) e aceita um
   acompanhante (`Tracer`), avisado antes de cada oração e na falha com a pilha à vista
   (`View`); é por ele que o depurador (`debug.rs`) para, anda e mostra. A oração que a
   língua desdobra em duas (o "o resultado" escondido, com o mesmo trecho) é avisada uma
   vez só. Sem acompanhante, o custo é um teste por oração.
6. `codegen.rs` + `runtime/lib.rs`: nativo (ver abaixo).

`lexicon.rs`: o léxico, fonte única das palavras da língua (artigos, contrações,
preposições, verbos da língua, verbos do sujeito, tipos, operações por extenso, ordinais,
locuções, particípios e as demais palavras-chave), cada uma com o papel e uma linha de
documentação. O parser, o lexer e a morfologia consultam as tabelas dele; as formas que um
verbo do usuário não pode ter na 1ª pessoa saem dele; `lavrac lexicon --json` exporta tudo
para `editors/lexicon.json`, que a gramática do VS Code, o conferidor da morfologia e o
site leem. Testes do léxico: toda palavra que o parser consulta (os literais de
`at_word`, `eat_word`, `expect_word`, `is_word`, comparações e padrões de `match`) está no
léxico; toda entrada tem documentação; o JSON exportado está em dia
(`LAVRA_BLESS=1 cargo test --lib lexicon` regrava).

`codes.rs`: os códigos dos diagnósticos, estáveis e num espaço só (`L0001` em diante),
independentes da categoria; cada ponto que cria um diagnóstico passa o código
(`Diagnostic::new(L0334, ...)`) e a categoria sai do registro. O cabeçalho mostra o código
(`Erro de escopo [L0334]: ...`); a falha declarada pelo programa (`Desisto com`) não tem
código. O runtime inclui o registro e recebe do nativo o número do código, para o erro de
execução sair igual nos dois back-ends.

`morphology/`: flexão, artigos e ordinais (com as tabelas do léxico) e as tabelas
conferidas contra o dicionário (ver [morphology.md](morphology.md)). `diagnostics.rs`: categorias e renderização no
estilo do rustc. `textfmt.rs`: formatação e mensagens de execução, compartilhadas pelo
interpretador e pelo runtime (a paridade vem daí).

## Operações embutidas e bibliotecas

`builtins.rs`: as operações que a própria língua faz e as bibliotecas declaram em Lavra
(`src/libraries/textos.lav`, embutida no compilador). "Uso a biblioteca de textos." é lido
antes da pré-varredura (`Parser::imports`): os tokens da biblioteca, com as posições na faixa
dela (a partir de `LIBRARY_BASE`, 1 GiB, fora de qualquer programa), passam pelo próprio
`header()`, e o cabeçalho que termina em ponto vira assinatura com `builtin`. A chamada na
HIR aponta para `Callee::User` ou `Callee::Builtin`. O interpretador chama
`builtins::interpret`; o nativo chama a função do runtime de `Builtin::symbol`, com a
assinatura tirada dos tipos dos argumentos e do valor, e o trecho da chamada (`lo`, `hi`) no
fim para relatar uma falha. As operações recebem emprestado e devolvem um valor novo (o teste
de `builtins.rs` confere).

Uma operação nova toca quatro lugares: o cabeçalho na biblioteca, a variante em `Builtin`
(com a biblioteca, o verbo e o símbolo), o `interpret` e a função `rt_*` em `runtime/lib.rs`.
Regência, tipos, posse, codegen e o editor saem do cabeçalho. Programa certo (com a
paridade) e errado, como toda mudança.

Nada do programa aponta para dentro de uma biblioteca: o diagnóstico perde o rótulo e a
correção que cairiam lá (`outside_libraries` em `analyze`), o aviso de função nunca chamada
não vale para o que vem de biblioteca, e o rename recusa os verbos dela.

## Posições

`span.rs`: `Span` é um intervalo de bytes; `LineIndex` converte byte em linha e coluna, em
caracteres para as mensagens e em unidades UTF-16 para o protocolo dos editores, e volta;
`SourceMap` dá a cada arquivo uma faixa própria de posições (o primeiro começa em 0), para
um `Span` identificar o arquivo sem levá-lo junto, como no rustc; o léxico aceita a base
do arquivo (`tokenize_at`). Toda oração da HIR leva o trecho de onde veio
(`hir::Stmt { kind, span }`), e a HIR leva também o trecho das constantes, os registros e
as enumerações, e o índice da assinatura de cada função.

## Tabelas de símbolos

`tables.rs`: a sema registra, nas próprias passadas, cada nome declarado (`Def`: função,
parâmetro, local, constante, registro, campo, enumeração, caso, "o resultado"), cada uso
(`Ref`: leitura, escrita, chamada), os escopos (um por bloco), o tipo de cada expressão e o
modo de cada argumento. O servidor de linguagem consulta essas tabelas (`def_at`,
`refs_of`, `type_at`, `visible_at`) e nunca resolve nomes de novo. Delas saem os avisos de
não uso (função nunca chamada, L0502; nome nunca usado, L0503), calculados depois da posse
e só num programa sem erro nenhum.

Cada uso guarda a forma em que foi escrito (`Form`): o nome como declarado, o singular de
"Para cada nota", o tipo no plural ("uma lista de contas"), ou a forma do verbo
(`ast::VerbForm`: 1ª pessoa, infinitivo, subjuntivo e infinitivo pessoal com o número,
adjetivo com gênero e número), que o parser grava em `ast::Call` nos quatro lugares em que
monta uma chamada. O uso `Mention` é o nome que reaparece na própria declaração (a tipagem
do cabeçalho, "sendo a quantia um inteiro"; o adjetivo de "Para o número ser par") e não
conta como uso. Os tipos citados pelo nome (`ast::Program::type_uses`, gravados em
`type_word`) e a construção de registro também viram uso, e os campos de um caso de
enumeração são definições (donas: o caso). O rename se apoia nisso tudo; uma construção
nova que registra os usos com a forma certa já sai renomeada.

## Análise tolerante a erro

`lavra::analyze(texto)` não para no primeiro erro; devolve todos os diagnósticos (na ordem
do texto), os tokens, a AST e a HIR. `lavra::compile` é o atalho que exige zero erros e é
o que os back-ends recebem.

- **Léxico**: o trecho recusado vira um token `Error` e a varredura segue; texto sem
  fechamento encerra a linha lógica; recuo inconsistente fica no bloco mais próximo abaixo.
- **Parser**: oração quebrada é relatada (menos quando tem token de erro, que o léxico já
  acusou) e vira `Stmt::Error`, sincronizada no fim da linha lógica; o bloco recuado dela é
  lido como filho, e a cadeia de "Senão" de um "Se" quebrado vai junto; guarda os nomes que
  ela declararia. Ponto final esquecido fecha o período num ponto virtual. Cabeçalho
  quebrado vira assinatura com o que se leu (parâmetros com tipo de erro, chamadas que
  consomem a oração); definição de registro ou enumeração quebrada fica com o que se leu;
  "Conforme" se recupera caso a caso.
- **Sema**: confere oração por oração. O nome de uma declaração que falhou existe com tipo
  de erro, e "o resultado" dela também; o tipo de erro passa por todas as conferências sem
  acusar nada. Função com erro (dela ou do cabeçalho) não dá avisos, não confere o retorno e
  não passa pela posse.
- **Posse**: relata cada erro uma vez e segue, com o nome de volta a vivo.

Critério: nenhum erro que sumiria ao corrigir um anterior. Os testes
`tests/programs/err/recuperacao_*` têm vários erros independentes cada um, e o teste de
robustez roda a análise completa nas versões mutiladas do corpus.

## Nativo

- `build.rs` compila `runtime/lib.rs` com o `rustc` do toolchain (staticlib, LTO,
  ~8 MB) e o `lavrac` embute o arquivo; `lavrac build` escreve objeto e runtime num
  diretório temporário e linka com `cc` (`-lgcc_s -lutil -lrt -lpthread -lm -ldl -lc`).
- O runtime inclui por `#[path]` os mesmos `span.rs`, `diagnostics.rs` e `textfmt.rs`, e o
  executável leva o fonte embutido: o erro de execução sai renderizado igual ao do
  interpretador, com o trecho do código.
- Escalares em variáveis SSA; local confiado vai para slot de pilha. Texto, lista,
  registro, enumeração e "talvez" são ponteiros para o monte; dentro deles, e em todo
  lugar endereçável, cada valor ocupa uma casa de 8 bytes. Um descritor de tipo estático
  (`Desc`) diz ao runtime como escrever, comparar, copiar e liberar.
- Bandeira de posse por local com dono: liberação determinística na saída da função, no
  `vira` e no redeclarar dentro de laço; temporários liberados no fim da oração.
- Estouro com `sadd_overflow` e família; divisão por zero, `MIN / -1`, deslocamento fora
  da largura, raiz de negativo e posição fora viram falha com mensagem.
- Função que pode falhar devolve também o ponteiro da falha (ou nulo).
- Profundidade de chamadas contada nos dois backends (10 000, mesma mensagem).

Desempenho medido (2026-10-22): contar os primos até 2 000 000 por divisão leva 0,35 s no
nativo e 28,7 s no interpretador.

Custo de cópia no interpretador: ler um lugar inteiro como valor clona o valor. Por isso
`o tamanho de` mede no próprio lugar (`len_at` sobre o caminho), sem ler a coleção; antes
disso, o crivo até 100 000 levava minutos e crescia de forma quadrática. Continua
proporcional ao tamanho, no interpretador, passar coleção, texto ou registro para uma
função: o argumento é copiado na ida e, se confiado, na volta. O nativo passa ponteiro e
não tem esse custo. Com a pilha na máquina, o empréstimo pode virar referência ao quadro
do chamador; é o próximo passo dessa conta.

## Releases

Cada tag `v*` gera a release no GitHub (`.github/workflows/release.yml`):

- o `lavrac` estático (musl) para x86_64 e aarch64, com o sha256, compilado no próprio runner
  pelo compilador cruzado da taiki-e (o `build.rs` compila o runtime para o mesmo alvo, e o
  `--version` traz o commit);
- a extensão do VS Code de cada plataforma (`lavra-vscode-<versão>-linux-x64.vsix` e
  `-linux-arm64.vsix`), com aquele `lavrac` dentro;
- a conferência do que se baixa (job `verify`): o checksum e todos os samples com a saída
  esperada, o aarch64 pelo QEMU e o x86_64 também compilado para nativo.

O `lavrac` musl roda em qualquer Linux. O `lavrac build` liga o programa com o `cc` do sistema
(gcc ou clang), e o runtime compilado para musl liga tanto com a glibc quanto com a musl:
conferido em Debian e Alpine limpos, com o corpus e os samples, antes da primeira release.

O release-plz (`.github/workflows/release-plz.yml`, `release-plz.toml`) mantém o PR de
release, com o CHANGELOG em português; só o merge publica (`release_always = false`), cria a
tag e a release e chama os binários. Enquanto a crate não está no crates.io, as versões vêm das
tags (`git_only = true`), porque no modo do registro o release-plz recusa a tag que o
registro não tem. Para passar a publicar no crates.io: o token como secret
`CARGO_REGISTRY_TOKEN`, um `cargo publish` da versão da última tag, e então
`git_only = false` com `semver_check = true`. As lojas de extensão publicam a cada release
quando os secrets `VSCE_PAT` (Visual Studio Marketplace, publisher `joaohenriquebarbosa`) e
`OVSX_PAT` (Open VSX) existem.

## Crescer a língua

### Lista de conferência de uma feature

Cada construção nova passa por estes lugares, nesta ordem, e só os que ela de fato toca:

- **Léxico** (`src/lexicon.rs`): cada palavra nova com o papel e a linha de documentação. As
  formas reservadas saem dele; regere `editors/lexicon.json` com `lavrac lexicon --json` (um
  teste confere que está em dia, e outro que o parser só consulta palavras do léxico).
- **Parser** (`src/parser/`): leia as palavras pelos auxiliares (`at_word`, `eat_word`,
  `expect_word`, `at_punct`, `expect_punct`) e as classes pelas consultas que registram o
  que esperam; assim o completar do editor já oferece a construção nova. O que o programa
  declara de novo (verbo, tipo, caso) entra no `Vocabulary` pelas pré-varreduras.
- **AST, sema e HIR** (`src/ast.rs`, `src/sema.rs`, `src/hir.rs`): tipos, concordância e
  mensagens que concordam em gênero e número com o nome de que falam. Cada nome declarado
  ou usado vai para as tabelas (`def`, `use_def`), e é delas que o hover, a navegação e o
  rename tiram tudo.
- **Posse** (`src/ownership.rs`): o que a construção entrega, empresta ou muda.
- **Interpretador e nativo** (`src/interp.rs`, `src/codegen.rs`, `runtime/lib.rs`).
- **Erros**: código novo em `src/codes.rs`, com a explicação em `src/explain/LXXXX.md` (o
  teste roda o exemplo errado e o certo).
- **Programas**: um certo em `tests/programs/ok` (com o `.stdout`, que a paridade confere
  no interpretador e no nativo) e um errado em `tests/programs/err`, com a mensagem
  esperada; `LAVRA_BLESS=1 cargo test --test golden` grava, e o diff se revisa.
- **Notas**: `notes/lavra.md`, com a seção "Mudanças" se a mudança for incompatível.

Ficam de fora da lista o servidor de linguagem e as gramáticas dos editores: o servidor
tira tudo da análise, e as gramáticas TextMate e tree-sitter tiram as palavras do léxico
regerado. Só mexem nelas as construções que pedem realce de estrutura, como foi o "Uso a
biblioteca de ..." (`editors/vscode/syntaxes/build.py`).

### Compatibilidade

O corpus (`tests/programs`, `samples/`) é a suíte de compatibilidade. Antes da 1.0, uma
mudança incompatível entra quando os programas do corpus e a seção "Mudanças" de
`notes/lavra.md` mudam no mesmo commit. Se a migração é mecânica, ela vira uma correção
(`Fix`) do diagnóstico novo, e `lavrac fix` a aplica em quem já escreveu do jeito antigo.
No back-end, o critério continua sendo a paridade: a mesma saída, byte a byte, no
interpretador e no nativo.

### Roteiros

Os pontos de extensão que ainda não têm uso real ficam descritos aqui, para a feature que
chegar começar do lugar certo.

- **Módulos do usuário.** As posições já comportam vários arquivos: o `SourceMap`
  (`src/span.rs`) dá a cada arquivo uma faixa própria, e um `Span` acha o arquivo sem
  levá-lo junto. O caminho é o das bibliotecas: o pedido do módulo é lido antes da
  pré-varredura (como `Parser::imports`), os tokens do arquivo entram com a base dele, os
  cabeçalhos, registros e enumerações vão para o mesmo `Vocabulary` (colisão é erro, como o
  L0175), e os corpos entram no mesmo `hir::Program`, com ids globais. Nome de campo de
  outro módulo não entra no `field_names` do parse local, porque ele muda o parse de "a soma
  de". No editor, a definição passa a apontar para o arquivo do módulo, e o servidor analisa
  também os arquivos que o documento aberto pede. A sintaxe fica para quando for desenhada.
- **Genéricos e traits.** Um `Ty::Param` existe só entre a sema e a monomorfização: a sema
  confere a função genérica uma vez e, a cada chamada, deduz os tipos dos parâmetros e pede
  a instância concreta, que vira um `hir::Func` comum. Nenhum back-end vê variável de tipo.
  É a hora de internar os tipos (um id por tipo no lugar de `Box` e `Arc`), porque as
  instâncias multiplicam as comparações.
- **Referências guardadas.** O empréstimo de hoje é lexical: o `Loan` de um "Para cada" ou
  de um "Conforme" vale do começo ao fim do bloco (`LoanKind::Loop` e `LoanKind::Match`).
  Uma referência guardada num nome entra como `LoanKind::Binding`, do mesmo jeito: vale da
  declaração até o fim do bloco em que o nome vive, e enquanto isso o dono não muda nem se
  entrega. É o modelo lexical, não o NLL do Rust (o empréstimo não acaba no último uso).
- **Tipos novos no runtime.** O nativo descreve cada tipo por um `Desc` estático
  (`runtime/lib.rs`), que o codegen escreve num lugar só (`desc` em `src/codegen.rs`, com o
  mesmo número de `KIND_*`). Um tipo novo (mapa, conjunto) ganha um `KIND_*` e o braço dele
  nas funções do runtime que olham o tipo: `format_into`, `rt_drop`, `is_copy`, `rt_clone`,
  `rt_cmp` e `rt_len`. As operações sobre ele entram como builtins de uma biblioteca.

## Testes

- `cargo test`: unitários de morfologia e léxico; golden (`tests/golden.rs`) sobre
  `tests/programs/ok`, `tests/programs/err` e `samples/`; paridade: todo programa que
  compila roda no interpretador e no nativo, e stdout, stderr e código de saída precisam
  ser idênticos.
- `LAVRA_BLESS=1 cargo test --test golden` regrava as expectativas.
- Robustez (`tests/robustness.rs`): cada programa do corpus é mutilado de formas
  determinísticas (cortado no fim de cada linha e no meio de palavras, sem uma palavra,
  sem uma linha, com uma linha repetida, com um caractere a menos ou um sinal a mais,
  com duas palavras trocadas), cerca de 16 mil versões. A análise nunca pode entrar em
  pânico nem travar, todo diagnóstico aponta para dentro do texto em fronteira de
  caractere, e duas rodadas dão o mesmo resultado. As falhas saem agrupadas pelo lugar do
  pânico, com o menor texto que o provoca. Na primeira rodada achou um laço infinito
  (parêntese sem par na busca do verbo do sujeito), um `unwrap` em "Para cada i:" sem
  coleção e um sublinhado que partia uma letra acentuada; os três viraram testes em
  `tests/programs/err`.
- `tools/memcheck.sh`: compila a suíte para nativo e roda sob valgrind (o do sistema, se
  houver; senão num contêiner efêmero `debian:trixie-slim`); acusa vazamento e acesso
  inválido.
- CI (`.github/workflows/ci.yml`): formato, clippy, `cargo test`, testes da extensão e,
  em cada push para `main`, o memcheck.
- `tools/morphology/`: verificação das tabelas contra o dicionário inteiro (ver
  [morphology.md](morphology.md)).
- `cargo clippy --all-targets -- -D warnings` e `cargo fmt` (`rustfmt.toml`, largura 140).
