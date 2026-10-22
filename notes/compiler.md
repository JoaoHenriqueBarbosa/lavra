# O compilador (`lavrac`)

Crate único em Rust 2024 (`Cargo.toml`): biblioteca `lavra` e binário `lavrac`. A feature
`native` (padrão) traz o Cranelift e o runtime; sem ela, a biblioteca (análise e
interpretador) compila para wasm32 (`cargo check --lib --no-default-features --target
wasm32-unknown-unknown`, conferido no CI), para o playground. A linha de comando é própria e
em português (o `clap` daria ajuda e erros de uso em inglês); `lavrac --version` mostra o
commit, e um pânico do compilador avisa que é defeito do compilador e pede o relato.
Dependências: o Cranelift 0.137 (`codegen`, `frontend`, `module`, `object`, `native`) e o
`serde_json` (exportação do léxico).

```
lavrac run    arquivo.lav          interpreta
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
   cópia na ida e na volta (equivale à referência, porque o empréstimo é exclusivo).
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
não tem esse custo.

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
