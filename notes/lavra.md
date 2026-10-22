# A Lavra (referência da língua)

"Um Rust com gramática e morfologia": compilada, segura e rápida, em que cada instrução é
uma oração em português e cada traço gramatical é verificado pelo compilador. Esta nota
descreve a língua como o compilador em Rust (`src/`) implementa.

## Princípios

- **A gramática é verificação.** Artigo marca nome, concordância de gênero e número é
  checada em artigo, verbo, particípio, adjetivo e comparação; contração é obrigatória
  ("de o" é erro: "do"); regência é a assinatura das funções.
- **Voz híbrida.** Os dados são sujeitos do que acontece com eles ("O saldo cresce 100");
  o programa narra em 1ª pessoa quando age no mundo ("Escrevo", "Leio", "Devolvo") e nos
  verbos que o usuário define ("Rendo o saldo").
- **Posse no lugar de memória.** Texto, lista, registro, enumeração e "talvez" têm dono e
  se movem; números, lógicos e caracteres se copiam. Nada de ponteiro, null ou lixo.
- **Nada de crash mudo.** Estouro, divisão por zero, posição fora e entrada inválida são
  falhas de execução com mensagem e o trecho do código, iguais no interpretador e no nativo.

## Declaração: o verbo diz se muda

```
A gravidade é 9,81.                         fixo, tipo inferido (real)
O saldo começa em 0.                        variável ("começa" promete mudança)
O limite, um inteiro de 32 bits, é 100.     aposto entre vírgulas anota o tipo
As notas são 7, 8,5 e 9.                    nome no plural é coleção (lista de reais)
As compras começam vazias.                  lista variável; tipo dos itens inferido no uso
A fila, uma lista de inteiros, começa com 3 e 8.   singular só é coleção com aposto
O apelido, talvez um texto, começa em nada.
```

- Mudar o que "é" dá **Erro de mutabilidade**. "Começa" que nunca muda dá **Aviso**.
- Inicialização obrigatória. Nome no plural precisa ser coleção; "As notas são 7." é uma
  lista de um item. "Vazio" concorda ("as compras começam vazias").
- Lista com inteiros e reais escritos ("7, 8,5 e 9") vira lista de reais.
- No nível de fora só há constantes ("A taxa é 0,05."), funções e tipos. O que o programa
  faz fica em "Para começar:".

## Mutação: o dado é o sujeito

```
A mensagem vira "pago".          O saldo cresce 100.        O saldo diminui 30.
A lista ganha 7.                 A lista perde o último.    As notas perdem a primeira.
Tomo a última das notas.         (tira com posse; o valor vai para "o resultado")
O item 2 das notas vira 9.       A primeira da fila cresce 1.
O saldo da carteira cresce 50.
```

O verbo concorda com o sujeito ("as notas ganham"). Texto não muda letra por letra.

## Tipos

`inteiro` (64 bits com sinal, ou `de 8/16/32 bits`), `natural` (sem sinal), `byte`,
`real` (64, ou `de 32 bits`), `lógico`, `caractere`, `texto`, `lista de Xs`, registros,
enumerações, `talvez um X`. Tipos de valor copiam; os demais têm dono.

## Lugares e acesso

O "de" genitivo se lê da direita para a esquerda: `o nome do titular da conta`.
Posições começam em 1 nos dois jeitos: `o item 1 das notas` e `a primeira das notas`
são o mesmo item; `a última das notas` é o último. Fora do intervalo é falha de execução.
Num texto, a posição dá o caractere. `o tamanho de X` vale para lista e texto.

## Registros

```
Uma conta é um registro de:
    o titular, um texto;
    o saldo, um real;
    as movimentações, uma lista de reais.

A carteira começa em uma conta com o titular "Ana", o saldo 100,0 e as movimentações 100,0.
```

Campo pode ser símbolo (`x, um real;`), lido como `o x de p`. Construção exige todos
os campos. Campo no plural com um valor só vira coleção de um item.
Registro que contém a si mesmo é erro (tamanho infinito). A escrita de um registro sai
por extenso: `uma conta com o titular Ana, o saldo 150 e as movimentações [100; 50]`.

## Enumerações e "Conforme"

```
Uma forma é uma destas:
    um círculo, com o raio, um real;
    um retângulo, com a base, um real, e a altura, um real;
    um ponto.

Conforme a figura:
    Sendo um círculo: devolvo 3,14159 vezes o raio vezes o raio.
    Sendo um retângulo ou um quadrado: ...
    Senão: ...
Se a figura for um ponto: ...
```

"uma destas"/"um destes" concorda com o tipo; cada caso tem o seu gênero ("uma círculo" é
erro). Num braço com um caso só, os campos viram nomes (empréstimos do valor). Sem
"Senão", faltar caso é erro; com todos tratados, "Senão" sobrando é aviso. Durante o
"Conforme", o valor está emprestado e não muda.

## Funções: regência como assinatura

```
Para elevar x a n, sendo x um real e n um natural, devolvendo um real:
Para transferir a quantia da origem para o destino,
        sendo a quantia um inteiro e a origem e o destino inteiros confiados:
Para contar os dias até a data, sendo a data uma data emprestada, devolvendo um inteiro:
Para o valor exceder o limite, sendo ambos inteiros, devolvendo um lógico:
Para o número ser par, sendo o número um inteiro, devolvendo um lógico:
Para trovejar (trovejo) ...:          forma da 1ª pessoa declarada (defectivos)
```

- Chamada como oração na 1ª pessoa (`Transfiro 300 da conta para a poupança.`) ou como
  valor (`o resultado de elevar 2 a 10`). Preposição e ordem são as do cabeçalho; senão,
  **Erro de regência** com a forma certa.
- Objeto com artigo e sem tipo é **complemento fixo**: faz parte do verbo
  (`contar os dias até`). Com complemento fixo no cabeçalho, `ambos`/`ambas` são os dois
  regidos (`Para medir o tempo desde a origem até a data, sendo ambas datas emprestadas`).
- Predicado (sujeito antes do verbo) vira condição no futuro do subjuntivo
  (`Se o saldo exceder o teto`); predicado adjetivo flexiona (`Se o pino for aceso`).
- `o fato de X ser ...` dá um lógico (infinitivo pessoal: "serem"); antes do sujeito de
  infinitivo não se contrai ("o fato de o", não "o fato do").
- Funções podem ser usadas antes de definidas. Proibido redefinir verbo da língua,
  predicado com "ir" (o subjuntivo "for" é da comparação), verbo cuja 1ª pessoa é palavra
  da língua ("como", "caso", "salvo", ordinais...), defectivo sem a forma declarada, e
  dois verbos com a mesma 1ª pessoa no mesmo programa.

## Posse e empréstimo

O particípio do cabeçalho declara a relação, concordando com o tipo:

| Cabeçalho | Relação | Rust |
|---|---|---|
| `sendo a carta um texto` | recebe a posse | `T` |
| `sendo a carta um texto emprestado` | empréstimo de leitura | `&T` |
| `sendo o saldo um real confiado` | empréstimo exclusivo, pode mudar | `&mut T` |

O verificador (`src/ownership.rs`) acompanha cada local pelos caminhos do programa:

```
Erro de posse: A carta foi entregue a "arquivar" na linha 6 e não volta.
  = Depois de entregue, "a carta" não se usa mais.
  = dica: Se "arquivar" só precisa ler, peça emprestado no cabeçalho
    ("sendo a carta um texto emprestado"), ou entregue uma cópia da carta:
    "Arquivo uma cópia da carta."
```

- Entregar: declarar outro nome com o valor, `vira`, `ganha`, parâmetro sem particípio,
  `Devolvo`. Usar depois é **Erro de posse**; num dos caminhos, "pode já ter sido
  entregue"; num laço, "numa volta anterior do laço".
- `uma cópia de X` é o clone explícito.
- Não se tira um dono de dentro de registro ou lista entregando (use cópia, ou `Tomo`);
  constante não se entrega.
- Na mesma chamada, o que vai confiado não vai de novo nem emprestado (**Erro de
  empréstimo**). Durante "Para cada", a coleção está emprestada ao laço e não muda; o item
  e o valor de "Se houver" são vistas emprestadas e não se entregam.
- Modelo atual: posse, empréstimo de parâmetro e vistas (sem referências guardadas em
  variáveis, sem tempos de vida anotados). Referências de primeira classe ficam para depois.

## Ausência e falha

```
O apelido, talvez um texto, começa em nada.
Se houver apelido: escrevo o apelido.       dentro do bloco, o valor presente
Se não houver troco: ...
Se o apelido for nada: ...

Para dividir a conta entre as pessoas, ..., devolvendo um inteiro, salvo uma falha:
    Se as pessoas forem menores que 1: desisto com "ninguém para dividir".
A parte é o resultado de dividir a conta entre as pessoas, ou desisto.   passa a falha adiante
Escrevo o resultado de dividir 100 entre 0, ou 0.                       valor de reserva
```

Chamar função que pode falhar sem dizer o que fazer é erro de compilação. "Para começar"
pode desistir; a falha não tratada encerra com `Falha: ...` e código 1.

## Controle e condições

`Se/Senão, se/Senão`, `Enquanto`, `Repito N vezes`, `Repito sempre`,
`Para cada i de A até B` (desce se A > B), `Para cada nota das notas`, `Para cada nota`
(acha "as notas" pelo plural; nome invariável como "lápis" exige a coleção explícita),
`Para cada letra do nome`, `Paro`, `Pulo`. Condições no futuro do subjuntivo com
concordância ("for"/"forem", "maior que"/"maiores que"); "e" antes de "ou"; "não" antes do
verbo. "é", "está", "há" numa condição são erro de morfologia com a forma certa.

## Expressões

Infixas com precedência: `mais`, `menos`, `vezes`, `dividido por`, `módulo`.
Por extenso, de aridade fixa: `a soma`, `a diferença`, `o produto`, `o quociente`,
`o resto`, `a potência`, `a interseção`, `a união`, `a disjunção` (de X e Y; "entre" só
nas comutativas), `o negativo`, `a raiz`, `o tamanho`, `o inverso`, `o módulo` (de X),
`o deslocamento de X por N à esquerda/à direita`. **Misturar por extenso com infixa
exige parênteses**: `(a diferença de 10 e 3) mais 1`. Texto se junta com `seguido de`
("mais" com texto é erro com dica).

Conversão com `como`, sem artigo: `o total como real`, `9,99 como inteiro` (descarta a
parte fracionária), `x como byte`, `42 como texto`. Valor que não cabe é falha de
execução ("Conversão impossível: 300 não cabe num byte."), nunca truncamento silencioso.
Os dois lados de uma conta têm o mesmo tipo; número escrito toma o tipo do outro lado
(`2 vezes a taxa`), nome não (`o total vezes a taxa` pede `o total como real`). `o resultado` retoma o valor da oração anterior do
mesmo período.

`seguido de` liga mais frouxo que tudo: num argumento de chamada ele não entra, então
`seguido do resultado de flexionar 3 entre "quadrado" e "quadrados" seguido de " de lado."`
junta o resultado ao resto da frase. Para juntar texto dentro de um argumento, parênteses.

## Entrada e saída

`Escrevo X seguido de Y.` (quebra a linha, salvo se a oração seguinte do período for
`Leio`), `Escrevo X sem quebrar a linha.`, `Leio o lugar.`, `Leio um inteiro.`
(o valor vai para "o resultado"). Lista sai como `[7; 8,5; 9]` (ponto e vírgula, porque a
vírgula é decimal); real com vírgula e até 12 algarismos; lógico como verdadeiro/falso.
`LAVRA_ECO=1` ecoa a entrada lida.

## Léxico

Períodos com ponto; blocos com dois-pontos e recuo; linha sem `.`, `:` ou `;` continua na
de baixo (enjambement). Comentários: linha com `Nota:` ou `--`. Números com vírgula
decimal e milhar com espaço (`1 000 000`), hexadecimal `0xFF`. Sinais como `+` e `=` são
erro léxico com a palavra certa na dica.
