# Programas-poema

A proposta do user: programas que sejam ao mesmo tempo **úteis, que compilem e que
sejam poesia**. Os versos funcionam porque o vocabulário técnico da Lavra (registro,
resto, resultado, cópia, emprestado, confiado) também é vocabulário comum, e o poema usa
as duas leituras ao mesmo tempo. Os cinco estão em `samples/`, com entrada e saída
esperadas, na suíte golden e na de paridade entre interpretador e nativo.

## Lavra dos dias (`samples/lavra-dos-dias.lav`)

Pergunta quando a pessoa nasceu e que dia é hoje; responde o dia da semana do
nascimento, quantos dias já viveu, se hoje é aniversário e quanto falta para o próximo
milhar de dias.

- **"Uma data é um registro de"**: struct, e registro de cartório.
- **`Devolvo uma data com o dia o sol, o mês a lua e o ano o tempo.`**: a data perguntada
  volta inteira, e o ciclo de sol, lua e tempo, que está no cálculo, vira verso.
- **`O tempo começa no ano da data.` / `A lua começa no mês da data.`**: declarações que
  se leem como frase.
- **"O resto ... e 7"**: dia da semana como resto da divisão por sete (`N é 1 mais o
  resto ... e 7`, porque as posições começam em 1).
- **`Se o nascimento rimar com o agora`**: aniversário como rima (o fim se repete, o
  resto muda); `sendo ambas datas emprestadas`, o empréstimo dentro do verso.
- O cálculo começa o ano em março (o 29 de fevereiro fica no fim, como no calendário
  romano; por isso setembro a dezembro são sétimo a décimo). "A lua diminui 3" põe o ano
  romano no programa. Termina na ladainha gregoriana (mais /4, menos /100, mais /400).
  O 153 é março a julho (31+30+31+30+31) e o número de peixes de João 21. A semana
  começa na terça porque o resto zero cai numa terça.

Recursos formais que viraram verso: **enjambement** (linha sem `.`, `:` ou `;`
continua, com recuo livre: os sete dias da semana na vertical); **complemento fixo**
("contar os dias até" é obrigatório em toda chamada; a repetição é exigência do
compilador); **a gramática protege o poema** ("a nascimento" ou "rima" no lugar de
"rimar" não compilam).

Conferido: 13/05/1990 a 22/10/2026 dá domingo e 13 311 dias; 29/02/2000 a 29/02/2024
dá terça-feira, 8 766 dias e o aniversário.

## A partilha (`samples/a-partilha.lav`)

Um agrimensor reparte a terra de um morto: corta quadrados do terreno até sobrar um
quadrado (algoritmo de Euclides, o máximo divisor comum como medida do lote), conta os
lotes, reparte entre os herdeiros, e o que "não se deixou partir" fica com a viúva.

- `Corto os quadrados da sobra no dia.` e `Giro a sobra.`: a sobra e o dia passam como
  parâmetros confiados, e a regência ("da sobra no dia") entra no verso.
- `A sobra começa numa cópia da herança.`: a cópia é dita.
- `Enquanto a sobra não for quadrada:` e `Se a herança for vazia:`: predicados
  adjetivos que flexionam ("o terreno ser quadrado" aceita "a sobra for quadrada").
- Os herdeiros são uma lista que ganha uma pessoa por nome lido.

Conferido com 13 por 21 braças e três herdeiros: seis dias de corte (no sétimo, sobra o
quadrado de 1 braça), 273 lotes, 91 para cada um, a viúva sem lote.

## Primeira Catilinária (`samples/catilinaria.lav`)

Templo de Júpiter Estator, 8 de novembro de 63 a.C. Cícero acusa, Catilina responde, o
senado é consultado a cada rodada (`s/n`); a paciência do senado decide se Catilina é
expulso ou se a acusação é retirada.

- Os oradores passam pela regência, em espelho: `Acuso o réu pelo cônsul com ...` e
  `Respondo ao cônsul pelo réu com ...`.
- `Consulto o senado sobre o cônsul e o réu.`: o senado, o cônsul e o réu vão confiados,
  e a credibilidade muda dentro da consulta.
- `Ameaço a cidade pelo réu e deixo a cidade com o réu.` e `Retiro a acusação pelo
  cônsul e permaneço com o réu.`: os dois desfechos.
- `Enquanto o senado for paciente e n for no máximo 4:`: predicado adjetivo.

## Ab urbe condita (`samples/ab-urbe-condita.lav`)

Conta o tempo desde a fundação de Roma (21 de abril de 753 a.C., data de Varrão) até a
reunião na casa de Marco Leca, na rua dos Foiceiros.

```
Para medir o tempo desde a origem até a data, sendo ambas datas emprestadas:
Meço o tempo desde a fundação até a reunião.
```

"Meço" sai da conjugação irregular; "o tempo" é complemento fixo; "desde ... até" é a
regência; "ambas" são os dois regidos. Os nomes da conta repetem os de Lavra dos dias:
a era, a lua, o sol. Saída:

```
Casa de Leca, rua dos Foiceiros, 63 a.C.:
    690 anos, 6 meses e 17 dias desde a fundação;
    é o ano 691 ab urbe condita.
O mesmo dia, em 68 a.C.:
    685 anos, 6 meses e 17 dias desde a fundação;
    é o ano 686 ab urbe condita.
Hoje:
    2778 anos, 6 meses e 0 dias desde a fundação;
    é o ano 2779 ab urbe condita.
```

Este programa é a origem da regra dos parênteses: "a diferença do ano da data e do ano
da origem mais 1" se lê de dois jeitos (e um deles dá 689). Misturar operação por
extenso com infixa sem parênteses é erro de compilação
(`tests/programs/err/ab_urbe_ambiguidade.lav`); a forma certa é `1 mais a diferença do
ano da data e do ano da origem`.

## O joio e o trigo (`samples/o-joio-e-o-trigo.lav`)

Crivo de Eratóstenes: pergunta até onde vai o campo, lista os primos até lá e diz quantos
grãos de trigo ficaram.

```
Separo o joio do trigo.
Colho o trigo.
```

- **`Para separar o joio do trigo, sendo o trigo uma lista de lógicos confiada:`**: "o
  joio" é complemento fixo e "do trigo" é a regência, então a parábola (Mateus 13) é a
  própria chamada. O trigo vai confiado porque a separação risca os múltiplos nele.
- **`Colho o trigo.`**: a colheita só lê, e o trigo vai emprestado.
- O campo nasce todo trigo (`O trigo ganha verdadeiro.`, uma vez por grão); o joio é o
  que a peneira risca (`O item m do trigo vira falso.`), e "o item 1 do trigo" é joio
  desde o começo.
- Fecha o ciclo da lavoura dos outros poemas: a terra repartida em *A partilha*, a foice
  dos Foiceiros, e agora a colheita.

Conferido contra Python de 0 a 100 000 (9 592 primos), com paridade entre interpretador e
nativo. Este programa revelou que `o tamanho de` copiava a coleção no interpretador
(ver [compiler.md](compiler.md)).

## Padrões emergentes

O user observou que, entre os programas, palavras e temas reapareceram sem plano ("um
registro de", "a lua", "o resto", a partilha, a foice dos Foiceiros), como tambores
sobrepostos ou linhas numa pintura abstrata. A leitura que ficou: a paleta pequena e
obrigatória de palavras acumula sentido de um texto para o outro; os programas são sobre
ciclos de tamanhos diferentes se sobrepondo (semana, mês, ano, MMC e MDC), como em
*Clapping Music* e *Drumming* de Steve Reich; e cada frase Lavra tem duas vozes, a
técnica (que o compilador exige perfeita) e a comum (livre, desde que não desafine a
primeira).
