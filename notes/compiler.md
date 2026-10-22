# O compilador (`lavrac`)

Crate único em Rust 2024 (`Cargo.toml`): biblioteca `lavra` e binário `lavrac`.
Dependências: só o Cranelift 0.137 (`codegen`, `frontend`, `module`, `object`, `native`).

```
lavrac run    arquivo.lav          interpreta
lavrac build  arquivo.lav [-o x]   executável nativo (padrão: o nome sem .lav)
lavrac check  arquivo.lav          só confere
```

## Pipeline

1. `lexer.rs`: tokens com spans, INDENT/DEDENT, enjambement, contrações desfeitas
   (cada metade guarda a forma escrita), locuções ("maior que", "igual a", "pelo menos").
2. `parser/` (descida recursiva): `mod.rs` (períodos, orações, condições, expressões),
   `header.rs` (pré-varredura e cabeçalhos de função; chamadas guiadas pela regência),
   `data.rs` (registros, enumerações, lugares, coleções, operações por extenso).
   Confere a gramática local: concordância verbal, contração, regência, ordinais.
3. `sema.rs`: escopos, concordância nominal com a declaração, tipos com inferência
   (variáveis de tipo unificadas e resolvidas no fim de cada função), mutabilidade,
   exaustividade do "Conforme". Baixa para `hir.rs`.
4. `ownership.rs`: posse e empréstimo, por fluxo sobre a HIR (Se, laços com duas
   passadas, Paro/Pulo).
5. `interp.rs`: interpretador da HIR, referência de semântica. Parâmetro confiado por
   cópia na ida e na volta (equivale à referência, porque o empréstimo é exclusivo).
6. `codegen.rs` + `runtime/lib.rs`: nativo (ver abaixo).

`morphology/`: artigos, contrações, ordinais e as tabelas conferidas contra o dicionário
(ver [morphology.md](morphology.md)). `diagnostics.rs`: categorias e renderização no
estilo do rustc. `textfmt.rs`: formatação e mensagens de execução, compartilhadas pelo
interpretador e pelo runtime (a paridade vem daí).

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

## Testes

- `cargo test`: unitários de morfologia e léxico; golden (`tests/golden.rs`) sobre
  `tests/programs/ok`, `tests/programs/err` e `samples/`; paridade: todo programa que
  compila roda no interpretador e no nativo, e stdout, stderr e código de saída precisam
  ser idênticos.
- `LAVRA_BLESS=1 cargo test --test golden` regrava as expectativas.
- `tools/memcheck.sh`: compila a suíte para nativo e roda sob valgrind num contêiner
  efêmero (`debian:trixie-slim`); acusa vazamento e acesso inválido.
- `tools/morphology/`: verificação das tabelas contra o dicionário inteiro (ver
  [morphology.md](morphology.md)).
- `cargo clippy --all-targets -- -D warnings` e `cargo fmt` (`rustfmt.toml`, largura 140).
