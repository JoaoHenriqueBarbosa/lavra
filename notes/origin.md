# Origem da Lavra

## O pedido

O user pediu uma linguagem de programação teórica, de baixo nível e compilada, cujo
ponto central fosse **trazer gramática** para o código, podendo ser em português. O
exemplo dele era informal:

```
Declaro função: soma que recebe um float64 "a" e um float64 "b". A função retorna:
produto de operação de adição entre "a" e "b"

Loop infinito:
Escrevo na tela "Bem vindo, digite a primeira nota" e guardo o valor em "primeira".
```

E deixou explícito: "não se prenda à gramática que eu escolhi, tenho certeza que você
chega a uma mais otimizada e padronizada ainda, eu quero gramática, sintática e
morfologia".

## A tese de design

Cada instrução é uma **oração** em português, e a gramática não é enfeite: cada traço
gramatical tem função no compilador.

- **Artigos** marcam o que é variável (nome sempre leva determinante).
- **Modo verbal** marca o papel da oração (infinitivo define, 1ª pessoa executa,
  futuro do subjuntivo é condição, gerúndio anota declaração).
- **Regência** marca os argumentos (a preposição faz parte da assinatura do verbo,
  como os seletores do Smalltalk).
- **Concordância** serve de verificação redundante contra erro.

Quatro mecanismos juntos eliminam a ambiguidade do parser (descendente recursivo,
LL(2) na intenção):

1. Determinante marca nome.
2. Verbo nu marca ação.
3. Modo marca papel.
4. Aridade fixa da forma nominal marca onde a expressão acaba.

O nome "Lavra" vem de *laborare*, trabalhar a terra. Extensão: `.lav`.

## Ordem dos acontecimentos (2026-10-21)

1. Especificação da linguagem (morfologia, sintaxe, semântica, EBNF, compilação),
   reproduzida em [language.md](language.md).
2. Implementação: léxico, morfologia, parser com verificações gramaticais e
   interpretador; depois o editor `lavra.html` (artefato publicado no claude.ai,
   "Apenas você"); depois o gerador x86-64 + `runtime.c` + CLI `lavrac.js`,
   empacotados no `lavra.zip`.
3. Segunda conversa: o user mandou o zip e pediu ajuda para pensar em aplicações
   ([applications.md](applications.md)).
4. O user propôs poesia: programas que sejam ao mesmo tempo úteis, que compilem e
   que sejam poemas. Saíram `lavra-dos-dias.lav`, `a-partilha.lav`,
   `catilinaria.lav` e um programa *ab urbe condita* ([poems.md](poems.md)).
5. Para a Catilinária compilar, o cabeçalho "Para responder **ao** cônsul" exigiu um
   patch no parser (contração no cabeçalho). Esse patch só existe nos arquivos soltos
   da raiz do `draft/` ([draft-status.md](draft-status.md)).

## O que o user disse sobre a direção

- Quer gramática, sintaxe e morfologia de verdade, não açúcar sintático.
- Viu na Lavra uma mídia artística: "a computação causou aqui efeito de padrões
  emergentes, com os tambores se sobrepondo, ou linhas em uma pintura abstrata".
- Frase dele que virou registro: "a língua que Lavra com foice de Leca que reparte
  com Catilina terra".
- Em 2026-10-22 pediu para explorar o draft, tomar notas e depois **refazer de acordo
  com as instruções dele** (instruções ainda não dadas no momento destas notas).
