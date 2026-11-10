# Mudanças

As versões da Lavra, a mais nova primeiro.

## [Unreleased]

### Novidades

- A função que pode falhar se declara com "salvo engano", o dizer corrente da língua. É a
  única forma: "engano" passa a ser palavra da língua, e "falha" deixa de ser.
- Tipo implícito no cabeçalho: quando o nome do parâmetro é o nome do tipo, o particípio vem
  direto ("sendo a prova emprestada" vale por "sendo a prova uma prova emprestada"). O rename
  escreve o tipo por extenso quando o nome ou o tipo muda.
- "talvez" no cabeçalho, no parâmetro e no retorno ("sendo o apelido talvez um texto
  emprestado", "devolvendo talvez um inteiro"): uma função já pode devolver "nada".
- "como" lê texto como número ("42" como inteiro, "3,5" como real) e vai do caractere ao
  código dele e de volta (a letra como inteiro, 65 como caractere). O que não se converte é
  falha de execução, com os códigos L0915 e L0916.
- Condições novas: "Se as compras forem vazias" (lista ou texto sem nada) e "Se houver 7 nas
  notas" (o item na lista, o trecho ou o caractere no texto).
- "Conforme" sobre valores: "Sendo 1 ou 7: ...", com inteiro, caractere, texto e lógico, e o
  "Senão" obrigatório.

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
