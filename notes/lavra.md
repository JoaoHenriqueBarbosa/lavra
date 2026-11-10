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
- **Nome composto com infinitivo**: "a intenção de continuar", "a sala de estar",
  "a máquina de lavar" são um nome só. O "de" sem contração seguido de infinitivo é
  complemento do nome, nunca genitivo (o genitivo pede artigo: "da conta") nem chamada
  ("o resultado de dobrar" e "o fato de" vêm antes nas expressões). Pode encadear
  ("a vontade de parar de fumar"); gênero e número vêm do artigo, e a flexão fica no
  núcleo ("as intenções de continuar"). O infinitivo se reconhece pela terminação
  (-ar/-er/-ir/-or), então "a dor de amor" também vale como nome composto.
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
`real` (64, ou `de 32 bits`), `lógico`, `caractere`, `texto`, `lista de Xs`, `conjunto de Xs`,
`mapa de Xs para Ys`, registros, enumerações, `talvez um X`. Tipos de valor copiam; os demais
têm dono.

## Conjuntos e mapas

```
Os vistos, um conjunto de textos, começam vazios.       sem repetição, em ordem
Os pares, um conjunto de inteiros, são 4, 2, 4 e 6.      {2; 4; 6}
Os vistos ganham "Ana".          Os vistos perdem "Ana".          (o próprio valor)
Se houver "Ana" nos vistos: ...  Para cada visto dos vistos: ...  (em ordem)
a união dos pares e dos primos   a interseção de A e B            a diferença de A e B

As idades, um mapa de textos para inteiros, começam vazias.
O item "Ana" das idades vira 30.        põe a chave, ou troca o valor dela
O item "Ana" das idades cresce 1.       Escrevo o item "Ana" das idades.
Se houver "Ana" nas idades: ...         As idades perdem "Ana".
Para cada nome das idades: ...          passa pelas chaves, em ordem
```

- O valor de um conjunto e a chave de um mapa são inteiro, caractere, texto ou lógico, os
  tipos com ordem (L0367); o valor de um mapa é de qualquer tipo, até outra coleção.
- A ordem é a dos valores (ou das chaves), não a de chegada: a escrita e o "Para cada" saem
  sempre iguais. Conjunto se escreve `{Ana; Bia}`, e mapa, `{Ana: 30; Bia: 25}`.
- Ler a chave que o mapa não tem é falha de execução (L0919); "vira" a põe no mapa. O que o
  conjunto ou o mapa perde e não estava lá não faz falta.
- Não têm posição: "a primeira de", "o item 2 de" e "Tomo" são da lista (L0368). O mapa
  começa sempre vazio. `o tamanho de` e `for vazio` valem para os dois.

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
Registro que contém a si mesmo é erro (tamanho infinito), a não ser dentro de uma lista, que
guarda os itens por fora: `as subpastas, uma lista de pastas` é campo de uma pasta, e é assim
que se fazem árvores. O mesmo vale para a enumeração (`uma adição, com as parcelas, uma lista
de expressões`) e para os ciclos entre dois tipos. "talvez" não quebra o ciclo. A escrita de um registro sai
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

"Conforme" também escolhe entre valores de um inteiro, um caractere, um texto ou um lógico:

```
Conforme o dia:
    Sendo 1 ou 7: escrevo "fim de semana".
    Sendo 2: escrevo "segunda".
    Senão: escrevo "dia útil".
```

O valor é lido uma vez. Como ele pode ser qualquer um, o "Senão" é obrigatório (L0366); só o
lógico dispensa, com "verdadeiro" e "falso" tratados. Real não entra (L0309).

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
Para apresentar a prova, sendo a prova emprestada:      o nome é o do tipo: tipo implícito
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
- **Tipo implícito**: quando o nome do parâmetro é o nome do tipo (registro, enumeração ou
  tipo da língua, como "o texto"), o particípio vem direto: `sendo a prova emprestada` vale
  por `sendo a prova uma prova emprestada`, e o mesmo com "confiada". Só no singular, um nome
  por vez, e só com particípio: o parâmetro de que a função toma posse continua
  `sendo a prova uma prova`. Nome que não é de tipo dá L0176.
- Predicado (sujeito antes do verbo) vira condição no futuro do subjuntivo
  (`Se o saldo exceder o teto`); predicado adjetivo flexiona (`Se o pino for aceso`).
- `o fato de X ser ...` dá um lógico (infinitivo pessoal: "serem"); antes do sujeito de
  infinitivo não se contrai ("o fato de o", não "o fato do").
- Funções podem ser usadas antes de definidas. Proibido redefinir verbo da língua,
  predicado com "ir" (o subjuntivo "for" é da comparação), verbo cuja 1ª pessoa é palavra
  da língua ("como", "caso", "salvo", ordinais...), defectivo sem a forma declarada, e
  dois verbos com a mesma 1ª pessoa no mesmo programa.

## Funções genéricas

```
Para inverter a lista, sendo a lista uma lista de coisas quaisquer emprestada,
        devolvendo uma lista de coisas:
    As invertidas, uma lista de coisas, começam vazias.
    ...
Para escolher entre o esquerdo e o direito, sendo ambos elementos quaisquer ordenáveis,
        devolvendo um elemento:
```

- "qualquer" depois de um nome, no cabeçalho, declara um **tipo qualquer**: "uma coisa
  qualquer", "uma lista de coisas quaisquer" (concorda em número, L0181). Daí em diante, no
  cabeçalho e no corpo, "coisa" é esse tipo, sem repetir "qualquer". O nome é livre
  ("coisa", "elemento", "item"), e o gênero vem do artigo.
- Cada chamada decide o tipo pelos argumentos: `inverter as notas` faz de "coisa" um
  inteiro, e `inverter os nomes`, um texto. O tipo que nenhum argumento decide é erro (L0370).
- De um valor de tipo qualquer a função sabe pouco, e só faz o que vale para todo tipo:
  entregar, emprestar, copiar ("uma cópia de"), comparar com "igual a" e "diferente de",
  escrever e juntar com "seguido de", guardar em lista. Conta, campo e posição não valem.
- "qualquer **ordenável**" pede um tipo com ordem (número, caractere ou texto) e libera
  "maior que" e "menor que"; chamar com outro tipo é erro (L0371).
- A função genérica é conferida uma vez, como foi escrita; o programa compilado leva uma
  cópia dela para cada combinação de tipos que usa.

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
- **Referência guardada num nome**: o mesmo particípio do cabeçalho, depois do valor de uma
  declaração, faz do nome uma referência ao lugar, em vez de dono de uma cópia:

  ```
  A favorita é a primeira das contas, confiada.     muda o que está lá (&mut)
  O saldo da favorita cresce 5.                      a conta da lista mudou
  O nome é o titular da conta, emprestado.           só lê (&)
  ```

  O particípio concorda com o nome declarado (L0179), a declaração é sempre com "é" (L0180),
  e o valor tem de ser um lugar: um nome, um campo ou um item (L0369). A referência vale da
  declaração até o fim do bloco em que o nome vive (não até o último uso): enquanto isso, o
  dono emprestado não muda nem se entrega (L0401, L0405), e o confiado nem se lê (L0407). A
  referência não se entrega, não se devolve e não se guarda em registro ou lista.
- Modelo atual: posse, empréstimo de parâmetro, vistas e referências guardadas num nome,
  todas com escopo de bloco (sem tempos de vida anotados, sem referência dentro de valor).

## Ausência e falha

```
O apelido, talvez um texto, começa em nada.
Se houver apelido: escrevo o apelido.       dentro do bloco, o valor presente
Se não houver troco: ...
Se o apelido for nada: ...

Para dividir a conta entre as pessoas, ..., devolvendo um inteiro, salvo engano:
    Se as pessoas forem menores que 1: desisto com "ninguém para dividir".
A parte é o resultado de dividir a conta entre as pessoas, ou desisto.   passa a falha adiante
Escrevo o resultado de dividir 100 entre 0, ou 0.                       valor de reserva
```

Chamar função que pode falhar sem dizer o que fazer é erro de compilação. "Para começar"
pode desistir; a falha não tratada encerra com `Falha: ...` e código 1.

No cabeçalho, "talvez" vem antes do tipo, no parâmetro e no retorno:

```
Para procurar o nome na lista, ..., devolvendo talvez um inteiro:    pode devolver "nada"
Para saudar o apelido, sendo o apelido talvez um texto emprestado:   o particípio segue o tipo
```

Quem chama passa o valor, "nada" ou um nome que já é "talvez"; o resultado se abre com "Se
houver". Só no singular: cada parâmetro que pode faltar tem a sua tipagem.

## Controle e condições

`Se/Senão, se/Senão`, `Enquanto`, `Repito N vezes`, `Repito sempre`,
`Para cada i de A até B` (desce se A > B), `Para cada nota das notas`, `Para cada nota`
(acha "as notas" pelo plural; nome invariável como "lápis" exige a coleção explícita),
`Para cada letra do nome`, `Paro`, `Pulo`. O item vem sem artigo, e o gênero dele sai, nesta
ordem, da coleção no plural ("Para cada questão das questões"), do tipo do elemento quando o
item tem o nome dele ("Para cada questão da lista", com "Uma questão é um registro"), e da
terminação ("-ção", "-são", "-dade", "-agem", "-a" femininos; o resto masculino, então
"Para cada canção" acerta e um "-ão" ambíguo pede a coleção no plural ou o tipo). Condições no futuro do subjuntivo com
concordância ("for"/"forem", "maior que"/"maiores que"); "e" antes de "ou"; "não" antes do
verbo. "é", "está", "há" numa condição são erro de morfologia com a forma certa.
`Se as compras forem vazias` e `Se o nome for vazio` perguntam se a lista ou o texto não têm
nada ("vazio" concorda com o sujeito; um predicado "ser vazio" do programa vem antes).
`Se houver 7 nas notas` procura o item na lista, e `Se houver "avr" no nome`, o trecho ou o
caractere no texto; com "não", `Se não houver "ovo" nas compras`. O "houver" sem artigo e sem
"em" continua sendo o do "talvez" (`Se houver apelido`).
Depois de "for"/"ser", "verdadeiro" e "falso" são predicativos e concordam com o sujeito
("a resposta for verdadeira", "as licenças forem falsas"; a discordância é L0137); como
valor ("vira verdadeiro", "devolvo falso"), são literais e não flexionam.

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
Texto se lê como número (`"42" como inteiro`, `"3,5" como real`, com os espaços das pontas
ignorados); texto que não é número do tipo pedido é falha de execução (L0915). Caractere e
inteiro vão um ao outro pelo código (`a letra como inteiro` dá 65 para "A", `65 como
caractere` dá "A"); número que não é código de caractere é falha (L0916).
Os dois lados de uma conta têm o mesmo tipo; número escrito toma o tipo do outro lado
(`2 vezes a taxa`), nome não (`o total vezes a taxa` pede `o total como real`). `o resultado` retoma o valor da oração anterior do
mesmo período.

`seguido de` liga mais frouxo que tudo: num argumento de chamada ele não entra, então
`seguido do resultado de flexionar 3 entre "quadrado" e "quadrados" seguido de " de lado."`
junta o resultado ao resto da frase. Para juntar texto dentro de um argumento, parênteses.

## Entrada e saída

`Escrevo X seguido de Y.` (quebra a linha, salvo se a oração seguinte do período for
`Leio`), `Escrevo X sem quebrar a linha.`, `Escrevo uma linha em branco.` ou `Escrevo uma
quebra de linha.` (as duas escrevem o texto vazio e quebram a linha: depois de uma escrita
sem quebra, só a terminam), `Leio o lugar.`, `Leio um inteiro.`
(o valor vai para "o resultado"). Lista sai como `[7; 8,5; 9]` (ponto e vírgula, porque a
vírgula é decimal); real com vírgula e até 12 algarismos; lógico como verdadeiro/falso.
`LAVRA_ECO=1` ecoa a entrada lida.

## Bibliotecas

```
Uso a biblioteca de textos.

Para começar:
    As partes são o resultado de dividir "a,b,c" por ",".
    Escrevo o resultado de juntar as partes com " | ".
    Se a frase contiver "lavra": escrevo o resultado de aparar a frase.
```

- "Uso a biblioteca de X.", no nível de fora, traz os verbos da biblioteca para o programa
  inteiro. Os verbos se usam como os do programa: na 1ª pessoa, no infinitivo depois de
  "o resultado de", no subjuntivo das condições, com a regência do cabeçalho.
- A biblioteca de textos traz "dividir o texto pelo separador" (o separador vazio dá cada
  caractere), "juntar os textos com o separador", "o texto conter o trecho" (o trecho vazio
  está em todo texto), "aparar o texto" (tira o espaço em branco do começo e do fim),
  "maiusculizar", "minusculizar" e "inverter o texto", "substituir o trecho pelo novo no
  texto", "localizar o trecho no texto" (a posição a partir de 1, ou nada: devolve "talvez um
  inteiro") e "recortar o texto desde o início até o fim" (posições a partir de 1, as duas
  incluídas; fora do texto é falha, L0917). Todas recebem emprestado e devolvem um valor novo.
- A biblioteca de matemática ("Uso a biblioteca de matemática.") traz operações por extenso,
  usadas pelo nome como as da língua: `o seno de x`, `o cosseno de x`, `a tangente de x` (em
  radianos), `o logaritmo de x` (natural; de zero ou negativo é falha, L0918), `a exponencial
  de x`, `o piso de x` e `o teto de x` (dão inteiro) e `o pi`. E três verbos: "arredondar x"
  (o inteiro mais próximo, o meio para longe do zero), "sortear desde o mínimo até o máximo"
  (um inteiro ao acaso, limites incluídos) e "semear a semente" (a mesma semente dá sempre os
  mesmos sorteios, no interpretador e no nativo). Como nas operações da língua, a conta infixa
  encostada pede parênteses: `(o seno de x) mais 1`; `o pi` não pede.
- A biblioteca de arquivos ("Uso a biblioteca de arquivos.") diz o arquivo pelo caminho, um
  texto: "carregar o arquivo" (o conteúdo, como texto), "gravar o texto no arquivo" (no lugar
  do que havia), "acrescentar o texto ao arquivo" (no fim), "apagar o arquivo", "o arquivo
  existir" (condição: `Se o caminho existir:`) e "listar a pasta" (os nomes, em ordem). O que
  depende do disco pode falhar ("salvo engano"), e a chamada diz o que fazer:
  `O conteúdo é o resultado de carregar "notas.txt", ou desisto.`,
  `Gravo o relatório em "saída.txt", ou desisto.` A falha diz o que não deu certo:
  `Falha: Não consegui carregar "notas.txt": não existe.`
- A biblioteca de tempo ("Uso a biblioteca de tempo.") trata o instante como um inteiro, os
  milissegundos desde o começo de 1970: `o agora` é o instante em que a oração roda;
  `o ano de x`, `o mês de x`, `o dia de x`, `a hora de x` e `o minuto de x` são as partes de um
  instante, no horário do lugar em que o programa roda (o segundo sai da conta, porque "o
  segundo de" já é a posição 2 de uma coleção: `o resto de (x dividido por 1 000) e 60`); "datar o
  instante" dá o texto "10/11/2026 14:57:03"; "aguardar o tempo" para o programa pelos
  milissegundos dados. Duração é conta de inteiros: `(o agora) menos o começo`.
- Operação por extenso de biblioteca só existe no programa que traz a biblioteca: sem ela,
  "o seno" é um nome qualquer. Com ela, "o pi" passa a ser a constante.
- Verbo de biblioteca não reserva palavra em programa que não a traz. No programa que a
  traz, definir um verbo com o mesmo nome (ou a mesma 1ª pessoa) é erro (L0175). Usar o verbo
  sem trazer a biblioteca dá a dica de como trazer.
- As bibliotecas são escritas em Lavra (`src/libraries/`): cada operação é um cabeçalho
  comum que termina em ponto, sem corpo, e a própria língua a faz.

## Arquivos trazidos

```
Uso o arquivo "contas.lav".            no nível de fora; o caminho conta da pasta deste arquivo
Uso o arquivo "../comum/formas.lav".
```

- O programa passa a ter as funções, os tipos e as constantes do arquivo trazido, como se
  estivessem escritos nele. Não há prefixo nem escolha do que trazer: vem tudo.
- O "Para começar" do arquivo trazido não entra. Assim cada arquivo pode ter o seu e rodar
  sozinho (para se testar), e o arquivo que só define nem precisa de um: `lavrac check` o
  aceita, e `lavrac run` avisa que ele não roda sozinho.
- Um arquivo trazido pode trazer outros e as bibliotecas de que precisa. Cada arquivo entra
  uma vez, por mais que seja pedido, e dois arquivos podem se trazer um ao outro.
- Definir nos dois lados o mesmo verbo com a mesma regência, ou o mesmo tipo, é erro, com o
  lugar da outra definição.
- O erro de dentro de um arquivo trazido aparece no arquivo dele, com a linha de lá; os
  avisos de lá ficam para quem roda aquele arquivo. A falha de execução também se mostra no
  arquivo em que aconteceu, no interpretador e no nativo.
- O pedido fora da forma é o L0177, e o arquivo que não se consegue ler, o L0178.

## Léxico

Períodos com ponto; blocos com dois-pontos e recuo; linha sem `.`, `:` ou `;` continua na
de baixo (enjambement). Comentários: linha com `Nota:` ou `--`. Números com vírgula
decimal e milhar com espaço (`1 000 000`), hexadecimal `0xFF`. Os inteiros também se escrevem
por extenso, de "zero" a 999 999: simples (`três`, `quinze`, `cem`, `mil`) e compostos como na
língua (`vinte e cinco`, `cento e vinte e três`, `dois mil e trezentos`, `mil duzentos e
trinta`). O composto é um número só, numa linha só; para a lista de 20 e 5, algarismos
(`20 e 5`). "um" e "uma" sozinhos continuam artigos: 1 se escreve `1` (dentro de um composto
valem, `vinte e um`). Os que flexionam vão no feminino antes de "vezes" (`duas vezes`,
`vinte e uma vezes`) e no masculino no resto (L0013). Um verbo do programa pode ter a
1ª pessoa igual a um numeral ("zerar" dá "zero"): o verbo abre a oração e o número nunca abre,
então `Zero o contador.` é a chamada. Sinais como `+` e `=` são
erro léxico com a palavra certa na dica.

## Mudanças

A Lavra ainda não tem versão estável; mudança incompatível entra quando o corpus inteiro
(`tests/programs` e `samples/`) muda no mesmo commit. Cada uma fica registrada aqui.

- **2026-11-10, numerais por extenso.** "zero", "dois" a "dezenove", as dezenas, as centenas
  e "mil" passam a ser palavras da língua, e não servem mais de nome ("o sete", "a dezena"
  continua livre) nem de 1ª pessoa de verbo. "vinte e cinco" passa a ser o número 25, e não a
  lista de 20 e 5. Nenhum programa do corpus usava essas palavras como nome.
- **2026-11-10, "salvo engano".** A função que pode falhar se declara com "salvo engano", e a
  locução anterior deixa de existir; "engano" passa a ser palavra da língua.
- **2026-10-22, palavras reservadas vindas do léxico.** A 1ª pessoa de um verbo do
  usuário não pode ser palavra da língua; a lista dessas palavras era mantida à mão e
  agora sai do léxico (`src/lexicon.rs`): toda palavra da língua e cada palavra das
  locuções, menos as formas recusadas com dica ("guardo", "seja", "há"), que continuam
  livres para o usuário. Conferido contra os 15 267 verbos do Verbiste: passam a ser
  proibidos só "emprestadar" (dá "emprestado") e "vaziar" (dá "vazio").
- **2026-10-22, avisos de não uso.** Função definida e nunca chamada (L0502) e nome
  declarado e nunca usado (L0503) viram avisos, só em programa sem erro. Os samples não
  têm nenhum; o "Se houver" e os campos que um "Conforme" expõe contam como uso.
- **2026-10-22, consertos achados pelas explicações dos erros.** Lista de enumeração
  ("uma lista de figuras") passa a ser tipo; registro e enumeração com o mesmo nome dão
  erro; a chamada sem preposição escolhe a definição que não pede preposição; "o resultado
  do dobrar" acusa o "de" contraído antes do verbo (L0134); predicado com "ir" dá o erro
  próprio (L0150); real grande demais para o tipo é erro (L0007), e não infinito.
- **2026-10-22, "uso" é palavra da língua.** "Uso a biblioteca de textos." traz uma
  biblioteca, e "usar" virou verbo da própria língua, narrado na 1ª pessoa como "Escrevo" e
  "Leio": o programa não define mais "Para usar ...". Nenhum programa do corpus usava.
