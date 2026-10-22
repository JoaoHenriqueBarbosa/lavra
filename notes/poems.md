# Programas-poema

A proposta do user: programas que sejam ao mesmo tempo **úteis, que compilem e que
sejam poesia**. Os versos funcionam porque o vocabulário técnico da Lavra (registro,
resto, resultado, endereço, apontado, inteiro) também é vocabulário comum, e o poema
usa as duas leituras ao mesmo tempo.

## Lavra dos dias (`draft/lavra-dos-dias.lav`)

Pergunta quando a pessoa nasceu e que dia é hoje; responde o dia da semana do
nascimento, quantos dias já viveu, se hoje é aniversário e quanto falta para o próximo
milhar de dias.

Sentidos duplos:

- **"Uma data é um registro de"**: struct, e registro de cartório.
- **"Seja o vivido um inteiro"**: declaração, e desejo de que o vivido seja inteiro.
- **"Pergunto pelo endereço do nascimento"**: passa ponteiro, e pergunta onde a
  pessoa nasceu. "O apontado pela data" é a memória e é o que a data indica.
- **"O resto ... e 7"**: dia da semana como resto da divisão por sete.
- **"Se o nascimento rimar com o agora"**: compara dia e mês sem o ano; aniversário
  como rima (o fim se repete, o resto muda).
- **"A lua" e "o tempo"**: mês e ano dentro da contagem. "Se a lua for menor que 3:
  subtraio 1 do tempo e acrescento 12 à lua".

O cálculo: ano começando em março (o 29 de fevereiro fica no fim do ano, como no
calendário romano, por isso setembro a dezembro são sétimo a décimo). "Subtraio 3 da
lua" põe o ano romano no programa. Termina na ladainha gregoriana (mais /4, menos
/100, mais /400) e em "mais o dia da data". O 153 é março a julho (31+30+31+30+31) e o
número de peixes de João 21. A semana começa na terça porque o resto zero cai numa
terça.

Recursos formais da gramática que viraram verso:

1. **Enjambement**: linha sem `.`, `:` ou `;` continua, com recuo livre (os sete dias
   da semana na vertical).
2. **Complemento fixo**: "contar os dias até" é obrigatório em toda chamada; a
   repetição é exigência do compilador.
3. **A gramática protege o poema**: "a nascimento" ou "rima" (em vez de "rimar") não
   compilam.

Conferido contra o calendário do Python: nascimento comum, 29/02/2000, aniversário no
dia, data no futuro ("Ainda não nasceste"), exatamente 10 mil dias (5/6/1999).

## A partilha (`draft/a-partilha.lav`)

Um agrimensor reparte a terra de um morto: corta quadrados do terreno até sobrar um
quadrado (algoritmo de Euclides, o máximo divisor comum como medida do lote), conta os
lotes, reparte entre os herdeiros com memória reservada (`Reservo a prole pessoas`), e
o que "não se deixou partir" fica com a viúva. Predicados adjetivos `ser quadrado` e
`ser vazio`; verbo `flexionar a quantidade entre o singular e o plural`.

## Primeira Catilinária (`draft/catilinaria.lav`)

Templo de Júpiter Estator, 8 de novembro de 63 a.C. Cícero acusa, Catilina responde, o
senado é consultado a cada rodada (`s/n`); a paciência do senado decide se Catilina é
expulso ou se a acusação é retirada. Registros `orador` e `assembleia`, predicado
`ser paciente`, verbos com regência (`Acuso o réu com ...`, `Respondo ao cônsul com
...`). Foi este programa que exigiu o patch de contração no cabeçalho
([draft-status.md](draft-status.md)).

## Ab urbe condita (só na conversa, não está no draft)

Conta o tempo desde a fundação de Roma (21 de abril de 753 a.C., data de Varrão) até
a reunião na casa de Marco Leca, na rua dos Foiceiros. Cabeçalho:

```
Para medir o tempo desde a origem até a data, sendo ambas datas:
```

Chamada: "Meço o tempo desde a fundação até a reunião." ("meço" sai da tabela de
irregulares; "o tempo" é complemento fixo; "desde ... até" é a regência). Saída
registrada:

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

O primeiro rascunho deu 689 por causa da ambiguidade nominal + infixa ("a diferença
do ano da data e do ano da origem mais 1"); a correção foi "1 mais a diferença ...".

## Padrões emergentes

O user observou que, entre os programas, palavras e temas reapareceram sem plano
("um registro de", "a lua", "o resto", a partilha, a foice dos Foiceiros), como
tambores sobrepostos ou linhas numa pintura abstrata. A leitura que ficou: a paleta
pequena e obrigatória de palavras acumula sentido de um texto para o outro; os
programas são sobre ciclos de tamanhos diferentes se sobrepondo (semana, mês, ano,
MMC/MDC), como em *Clapping Music* e *Drumming* de Steve Reich; e cada frase Lavra tem
duas vozes, a técnica (que o compilador exige perfeita) e a comum (livre, desde que não
desafine a primeira).

## Lavra dos dias na língua nova (`samples/lavra-dos-dias.lav`, 2026-10-22)

Reescrito como teste de regressão poética; a saída é idêntica à do protótipo (conferido
com 13/05/1990 a 22/10/2026 e com 29/02/2000 a 29/02/2024) e roda igual no nativo.

- **Perdeu**: "Pergunto pelo endereço do nascimento" e "o apontado pela data". Sem
  ponteiros, o duplo sentido de "endereço" (onde a pessoa nasceu) saiu do texto.
- **Ganhou**: a data agora é perguntada e devolvida inteira, `Devolvo uma data com o dia o
  sol, o mês a lua e o ano o tempo.`; o ciclo de sol, lua e tempo, que já estava no
  cálculo, virou verso. `O tempo começa no ano da data.` e `A lua começa no mês da data.`
  são declarações que se leem como frase.
- A contagem da semana passou a ser `1 mais o resto ... e 7`, porque as posições começam
  em 1; a regra dos parênteses não pesa, porque a forma por extenso fica no fim.
- `Para a data rimar com a outra, sendo ambas datas emprestadas`: o empréstimo entrou no
  verso; "emprestadas" concorda com "datas".
