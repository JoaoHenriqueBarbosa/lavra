# Origem da Lavra

## O pedido

O user pediu uma linguagem de programação compilada cujo ponto central fosse **trazer
gramática** para o código, em português. O exemplo dele era informal:

```
Declaro função: soma que recebe um float64 "a" e um float64 "b". A função retorna:
produto de operação de adição entre "a" e "b"
```

E deixou explícito: "não se prenda à gramática que eu escolhi, tenho certeza que você
chega a uma mais otimizada e padronizada ainda, eu quero gramática, sintática e
morfologia".

## A tese de design

Cada instrução é uma **oração** em português, e a gramática não é enfeite: cada traço
gramatical tem função no compilador.

- **Artigos** marcam o que é nome ("o saldo"); símbolo é uma letra sem artigo ("x").
- **Modo verbal** marca o papel da oração: infinitivo define ("Para medir a carta"),
  1ª pessoa executa ("Meço a carta."), futuro do subjuntivo é condição ("Se o saldo
  exceder o teto"), gerúndio anota o cabeçalho ("sendo", "devolvendo").
- **Regência** marca os argumentos: a preposição faz parte da assinatura do verbo, como
  os seletores do Smalltalk.
- **Concordância** é verificação redundante contra erro: artigo, verbo, particípio,
  adjetivo e comparação concordam com o nome de que falam.

O nome "Lavra" vem de *laborare*, trabalhar a terra. Extensão: `.lav`.

## A forma atual (decidida em 2026-10-22)

O mote é **"um Rust com gramática e morfologia"**: útil, segura e rápida. Decisões
tomadas com o user:

- Compilador em Rust, com interpretador (referência de semântica) e backend nativo
  Cranelift; paridade byte a byte entre os dois é o critério de correção.
- **Voz híbrida**: os dados são sujeitos do que acontece com eles ("O saldo cresce
  100"); o programa narra em 1ª pessoa quando age no mundo ("Escrevo", "Leio") e nos
  verbos que o usuário define.
- **Posse e empréstimo** como modelo de memória, com particípios no cabeçalho
  ("emprestado", "confiado"); sem null ("talvez", "nada"); falha declarada ("salvo uma
  falha", "Desisto com", ", ou desisto").

A referência da língua está em [lavra.md](lavra.md); o compilador, em
[compiler.md](compiler.md).

## O que o user disse sobre a direção

- Quer gramática, sintaxe e morfologia de verdade, não açúcar sintático.
- Viu na Lavra uma mídia artística: "a computação causou aqui efeito de padrões
  emergentes, com os tambores se sobrepondo, ou linhas em uma pintura abstrata".
- Frase dele que virou registro: "a língua que Lavra com foice de Leca que reparte
  com Catilina terra".
