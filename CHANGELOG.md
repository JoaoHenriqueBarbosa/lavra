# Mudanças

As versões da Lavra, a mais nova primeiro.

## [0.1.0] - 2026-10-22

### Novidades

- A língua: cada instrução é uma oração em português; o artigo marca o nome, o verbo diz se
  o dado muda ("é" ou "começa em"), a regência é a assinatura das funções e a concordância é
  conferida em artigo, verbo, particípio, adjetivo e comparação.
- Posse e empréstimo ("emprestado", "confiado"), "talvez" e falha ("salvo uma falha",
  "Desisto com"), registros, enumerações com "Conforme", listas e textos.
- A biblioteca de textos ("Uso a biblioteca de textos."): dividir, juntar, conter e aparar.
- O interpretador e o compilador para executável nativo (Cranelift), com a mesma saída byte a
  byte e as falhas de execução com o trecho do código.
- Diagnósticos com código estável, explicação (`lavrac explain`), correções seguras
  (`lavrac fix`) e saída em JSON (`lavrac check --json`).
- O formatador (`lavrac fmt`), que nunca muda quebra de linha, porque os poemas também são
  programas.
- O servidor de linguagem (`lavrac lsp`): erros ao digitar, realce semântico, hover,
  navegação, completar guiado pelo parser, assinatura, inlay hints, ações rápidas, rename
  com morfologia, dobra e seleção.
- O depurador (`lavrac dap`): pontos de parada, passos, pilha, variáveis, parada nos erros
  de execução e o terminal integrado.
- A extensão do VS Code, com o `lavrac` dentro.
