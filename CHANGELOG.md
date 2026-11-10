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
- Numerais por extenso, de "zero" a 999 999, simples e compostos: "Repito três vezes",
  "vinte e cinco", "dois mil e trezentos". Concordam antes de "vezes" ("duas vezes").
- Registro e enumeração recursivos por lista: "as subpastas, uma lista de pastas" é campo de
  uma pasta, e "uma adição, com as parcelas, uma lista de expressões" é caso de uma expressão.
- A biblioteca de textos ganhou maiusculizar, minusculizar, inverter, substituir, localizar
  (que devolve "talvez um inteiro") e recortar.
- A biblioteca de matemática ("Uso a biblioteca de matemática."): "o seno de x", "o cosseno",
  "a tangente", "o logaritmo", "a exponencial", "o piso", "o teto" e "o pi", como operações por
  extenso, e os verbos arredondar, sortear e semear.
- A biblioteca de arquivos ("Uso a biblioteca de arquivos."): carregar, gravar, acrescentar,
  apagar, listar e "Se o caminho existir". O que depende do disco pode falhar, e a chamada
  trata com ", ou desisto" ou ", ou VALOR".
- Funções genéricas: "sendo a lista uma lista de coisas quaisquer" declara um tipo qualquer,
  que cada chamada decide pelos argumentos. "qualquer ordenável" pede um tipo com ordem e
  libera "maior que".
- Referências guardadas num nome: "A favorita é a primeira das contas, confiada." muda a conta
  que está na lista, e ", emprestada" lê sem copiar. Valem até o fim do bloco, e o
  verificador de posse confere o dono enquanto isso.
- Arquivos trazidos: `Uso o arquivo "contas.lav".` traz as funções, os tipos e as constantes
  de outro arquivo Lavra. Cada arquivo pode ter o seu "Para começar" e rodar sozinho, e o
  erro de um arquivo trazido aparece nele.
- Conjuntos e mapas: "um conjunto de textos" (sem repetição, em ordem; ganha e perde o
  próprio valor; união, interseção e diferença) e "um mapa de textos para inteiros" ("O item
  "Ana" das idades vira 30."), com "Se houver ... em", "Para cada" e "o tamanho de".
- A biblioteca de tempo ("Uso a biblioteca de tempo."): "o agora", as partes de um instante
  ("o ano de x", "a hora de x"...), "datar o instante" e "aguardar o tempo".

### Desempenho

- O interpretador não copia mais o valor emprestado ou confiado a uma função: ele chega como
  endereço na pilha. Uma chamada com uma lista de 200 000 itens ficou mais de mil vezes mais
  rápida.

### Correções

- O nativo não perde mais memória quando o valor de reserva de uma chamada é um nome
  (", ou as linhas"): o nome continua dono do valor dele, e o resultado fica com uma cópia.
- Um verbo do programa pode ter a 1ª pessoa igual a um numeral ("zerar" dá "zero"): o verbo
  abre a oração, e o número nunca abre.

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
