# A linguagem, como está no draft

Referência da Lavra tal como `draft/lavra.js` implementa. Onde o draft diverge do
design original (primeira conversa), está marcado. O EBNF de design está no fim.

## 1. Léxico

- Comentários: linha que começa com `Nota:` (qualquer caixa) ou `--`. `--` no meio
  da linha também corta o resto.
- **Toda palavra é baixada para minúscula** (o design dizia só a primeira do período).
- Palavras: letras (com acento), dígitos, `_`, e hífen interno (`o contador-global`,
  `a soma-pares`, `segunda-feira`).
- Números: vírgula decimal colada (`3,14`), milhar com espaço em grupos de 3
  (`1 000 000`), sinal negativo literal (`-7`), hexadecimal (`0xFF`, extra do draft).
- Texto entre aspas duplas com escapes `\n`, `\t`, `\"`; caractere entre aspas
  simples (`'ç'`, `'\n'`, `'\0'`). Aspas tipográficas são normalizadas.
- Pontuação significativa: `. , : ; ( )`. `?`, `!`, `+ - * / = < >` fora de texto
  são erro léxico com dica ("Operações se escrevem por extenso").
- Fim de linha só fecha período se a linha termina em `.`, `:` ou `;`. Senão a linha
  continua na de baixo (enjambement) e o recuo dela é livre.
- Indentação gera INDENT/DEDENT (estilo Python); tab vira 4 espaços.

### Duas passagens antes do parser

1. **Locuções protegidas** viram um token: `pelo menos`, `no máximo`, `à esquerda`,
   `à direita` (antes das contrações, porque `pelo menos` não é `por o menos`).
2. **Contrações desfeitas**: `do/da/dos/das` = de + art, `no/na/nos/nas` = em + art,
   `ao/à/aos/às` = a + art, `pelo/pela/pelos/pelas` = por + art, `num/numa` = em +
   um/uma, `dum/duma` = de + um/uma. Cada metade guarda `contr` e `parte` (1 ou 2).
3. **Locuções** viram um token: `maior que`, `menor que`, `igual a`, `diferente de`,
   `dividido por`, `seguido de`, `seguida de`. A preposição final pode vir contraída
   (`igual ao`, `diferente do`, `dividido pelo`).

## 2. Morfologia

### Três famílias que não se confundem

- **Nome**: sempre com determinante, pelo menos 2 letras, gênero e número fixados
  pelo artigo na declaração. Não pode ser palavra reservada nem ordinal.
- **Símbolo**: uma letra minúscula, com dígitos opcionais, sem artigo e sem gênero
  (tratado como masculino singular). Regex: `^[b-df-np-z][0-9]*$|^[aeo][0-9]+$`, ou
  seja `a`, `e`, `o` sozinhos não são símbolos (são artigo e conjunção), mas `a1` é.
- **Verbo**: nu, sem determinante. Verbos-núcleo: `seja/sejam`, `guardo`,
  `acrescento`, `subtraio`, `multiplico`, `divido`, `escrevo`, `leio`, `devolvo`,
  `repito`, `paro`, `pulo`, `reservo`, `libero`.

Reservadas (não podem nomear nada): nomes das operações, nomes de tipos (singular e
plural), `deslocamento`, `resultado`, `fato`, `item`, `apontado`, `verdadeiro`,
`falso`, `nada`, `registro`.

### Modo verbal é função

| Forma | Papel |
|---|---|
| Infinitivo | define função (`Para somar ...`) ou usa o retorno como valor (`o resultado de somar ...`) |
| 1ª pessoa do presente | executa (`Somo 2 e 3.`) |
| Futuro do subjuntivo | condição (`Se a nota for ...`, `Se o saldo exceder o teto`) |
| Gerúndio | anota declaração (`valendo`, `sendo`, `devolvendo`) |
| Infinitivo pessoal | dentro de `o fato de` (`o fato de a prova ser maior que 7`) |

### Conjugação automática (1ª pessoa)

- Regular: tira `-ar/-er/-ir`, põe `-o`.
- Ajustes: `c→ç` (conhecer/conheço), `g→j` (proteger/protejo), `gu→g`
  (erguer/ergo), `-ear→-eio` (rastrear/rastreio), `-air→-aio`, `-por/-pôr→-ponho`.
- Alternância vocálica em `-ir`: `e→i` antes de rt/rv/rn/st/nt/gu/r/t/d
  (transferir/transfiro, seguir/sigo, servir/sirvo), `o→u` antes de rm/br/ss/l
  (dormir/durmo, cobrir/cubro).
- Tabela `IRREG_1PS`: ser, estar, ter, ir, vir, fazer (e derivados), dizer, trazer,
  ler, crer, ver (e derivados), medir, pedir (e derivados), ouvir, perder, poder,
  saber, querer, requerer, caber, valer, dar, haver, conter/manter/obter/deter/reter/
  entreter, intervir/convir/provir, odiar/ansiar/remediar/incendiar/mediar/
  intermediar, rir/sorrir.
- Verbo inventado irregular: forma declarada entre parênteses,
  `Para fronder (frondo) x:`.
- Proibido definir verbo cujo infinitivo é núcleo ou cuja 1ª pessoa colide com núcleo.

### Futuro do subjuntivo

Regular = infinitivo. Tabela `IRREG_SUBJ`: ser/ir→for, ter→tiver, estar→estiver,
fazer→fizer, poder→puder, caber→couber, saber→souber, querer→quiser, trazer→trouxer,
dizer→disser, vir→vier, ver→vir, rever→revir, prever→previr, haver→houver, dar→der,
derivados de ter e vir; `-por→-puser`.

### Plural

`-ão→-ões`, vogal `+s`, `-m→-ns`, `-al→-ais`, `-el→-éis` (ou `-eis` se já acentuado),
`-ol→-óis`, `-ul→-uis`, `-il→-is` (ou `-eis` se paroxítona acentuada), `-r/-z→+es`,
`-ês/-és/-ás/-ós→-eses...`, `-s/-x` invariável, compostos com hífen pluralizam cada
parte. Usado para ligar `Para cada nota:` a `as notas` e para o plural dos registros
(`Um orador` → `oradores`).

### Flexão de adjetivo predicativo

Base escrita no gênero do sujeito da definição; flexiona `-o↔-a`, `-ês→-esa`, e
plural. `Para a posição ser acesa` aceita `Se i for aceso`.

### Ordinais

`primeiro` até `décimo`, mais `último`, com gênero e número. Base 1. Concorda com a
coleção (`a primeira das notas`, não `o primeiro`).

## 3. Sintaxe

### Período, oração, bloco

- Programa = texto de períodos. Período = orações coordenadas por `e`, `,` ou `, e`,
  terminando em ponto. A coordenação só acontece se depois do conectivo vier um verbo.
- Cabeça de bloco termina em `:`. Bloco recuado na linha de baixo, ou uma oração na
  mesma linha (`Se ...: paro.`).
- Ordem fixa: verbo primeiro, sujeito oculto ("eu").
- Código no nível de fora (fora de funções) executa antes de `Para começar`. Um
  programa pode não ter `Para começar` (`script.lav`).

### Anáfora "o resultado"

Pronome que retoma o valor da oração anterior **no mesmo período**. Quem produz
valor: `leio`, `reservo` (endereço), `guardo` (valor guardado), ajustes (valor
final), chamada que devolve. `escrevo`, `seja`, `libero`, `paro`, `pulo` zeram. No x86
é literalmente `rax`/`xmm0`, salvo num slot só quando uma oração seguinte usa.

### Declaração

```
Seja o contador um inteiro, valendo 0.
Sejam a prova e o trabalho reais.            (composto: Sejam + tipo plural sem artigo)
Seja a gravidade, fixa, um real, valendo 9,81.
Sejam as notas uma sequência de 5 reais, valendo 7, 8,5, 6, 9,25 e 10.
Sejam os valores uma sequência de 4 inteiros de 16 bits, valendo 7.   (preenche tudo)
```

- `Seja` vs `Sejam` concorda com o sujeito; `fixo/fixa/fixos/fixas` concorda e
  exige valor.
- **Nome plural é coleção** (só sequência), nome singular nunca é sequência.
- Indefinido no sujeito é erro ("o indefinido vai no tipo").

### Registros

```
Uma conta é um registro de:
    o saldo, um real;
    o número, um natural de 32 bits;
    o titular, um endereço de caractere.
```

Artigo indefinido fixa o gênero do tipo novo; campos separados por `;`, último em `.`.
Campos podem ser símbolos (`x, um real;`). Registro que contém a si mesmo é erro (use
endereço). Registros são copiados por valor.

### Funções

```
Para elevar x a n, sendo x um real e n um natural, devolvendo um real:
Para medir a distância entre p e q, sendo p e q pontos, devolvendo um real:
Para o valor exceder o limite, sendo ambos inteiros, devolvendo um lógico:
Para o número ser par, sendo o número um inteiro, devolvendo um lógico:
Para fronder (frondo) x, sendo x um inteiro:
```

- Objetos diretos primeiro (coordenados por `e`/`,`), depois grupos `PREP param`.
  `entre X e Y` (ou qualquer `PREP X e Y`) é regência de dois termos.
- `sendo` tipa: grupos `x um real`, `p e q pontos`, `ambos/ambas` (concorda: `ambas`
  só se os dois forem nomes femininos).
- **Complemento fixo** (extra do draft): objeto com artigo e sem tipo vira parte do
  nome do verbo. `Para contar os dias até a data` exige `contar os dias até ...` em
  toda chamada.
- **Predicado** (sujeito antes do infinitivo): precisa devolver lógico; vira verbo de
  condição no subjuntivo. Com `ser ADJ` (extra do draft) vira predicado adjetivo.
- Chamada como oração: `Elevo 2 a 10.` Como valor: `o resultado de elevar 2 a 10`.
- A preposição da chamada tem que bater com a da definição: erro de regência com a
  forma esperada (`transfiro QUANTIA de ORIGEM para DESTINO`).
- Funções e registros podem ser usados antes de definidos (pré-varredura).
- `Para começar:` é a entrada, sem parâmetros e sem retorno.

### Controle de fluxo

```
Se COND:  /  Senão, se COND:  /  Senão:
Enquanto COND:
Repito sempre:       Repito N vezes:
Para cada i de A até B:        (desce se A > B)
Para cada nota das notas:      Para cada nota:   (acha "as notas" pelo plural)
Paro.   Pulo.                  (só dentro de laço)
```

O item de `Para cada nota` é um **alias** do elemento (modificar a nota modifica a
sequência: `Multiplico a nota por 2` dobra as notas).

### Condições

Orações com verbo no subjuntivo. `e` tem precedência sobre `ou`. `não` antes do verbo.

```
for maior que X / menor que X / igual a X / diferente de X / pelo menos X / no máximo X
for nada / for verdadeiro / for falso
SUJEITO PREDICADO_SUBJ argumentos     (Se o saldo exceder o teto)
SUJEITO for ADJ argumentos            (Se i não for par)
```

Lógico sozinho não é condição: tem que ser `for verdadeiro`/`for falso`.
`é`, `está`, `seja`, `fosse` numa condição viram erro de morfologia com a correção.

### Expressões

**Forma nominal** (aridade fixa, prefixa, sem precedência):
`a soma`, `a diferença`, `o produto`, `o quociente`, `o resto`, `a potência`,
`a interseção` (&), `a união` (|), `a disjunção` (^) com `de X e Y`; `o negativo`,
`o inverso` (~), `a raiz`, `o tamanho` com `de X`; `o deslocamento de X por N à
esquerda|à direita`. `entre` no lugar de `de` só para comutativas. O segundo termo
pode repetir `de` (`a diferença do x de p e do x de q`).

**Forma infixa**: `mais`, `menos`, `vezes`, `dividido por`, `módulo`, com precedência
usual e parênteses. `mais` com texto concatena (`"texto: " mais 10`).

**Acesso a dados** (o "de" genitivo lê da direita para a esquerda):

```
o saldo da conta            conta.saldo
a terceira das notas        notas[2]        (ordinal, base 1)
o item i das notas          notas[i]        (base 0)
o endereço do contador      &contador
o apontado pelo cursor      *cursor
x como inteiro de 32 bits   conversão (também "como texto", "42" como inteiro)
o fato de X ser ...         valor lógico
```

### Saída e entrada

- `Escrevo X seguido de Y ...` escreve com quebra de linha, exceto quando a oração
  seguinte do mesmo período é `leio` (vira prompt).
- `Leio ALVO` ou `Leio um TIPO` (valor vai para "o resultado"). Lógico aceita
  `verdadeiro/v/sim/s` e `falso/f/não/n`; real aceita vírgula.

### Memória

```
Reservo 256 bytes e guardo o resultado no buffer.
Reservo um nó e guardo o resultado no novo.        (singular para uma unidade)
Reservo a prole pessoas e guardo o resultado na descendência.
Guardo 0 no apontado pelo cursor.
Acrescento 1 ao cursor.                            (escala pelo tamanho do apontado)
Libero o buffer.
```

Diferença de dois endereços dá a contagem de itens. Endereço de byte é compatível com
qualquer endereço.

### Atribuição e ajuste

```
Guardo X em ALVO.        Acrescento X a ALVO.     Subtraio X de ALVO.
Multiplico ALVO por X.   Divido ALVO por X.
```

## 4. Tipos

| Tipo | Gênero | Padrão | Variações |
|---|---|---|---|
| `inteiro` | m | 64 bits com sinal | `de 8/16/32/64 bits` |
| `natural` | m | 64 bits sem sinal | idem |
| `real` | m | 64 bits | `de 32 bits` |
| `byte` | m | 8 bits sem sinal | |
| `lógico` | m | 1 byte | |
| `caractere` | m | 32 bits (Unicode) | |
| `texto` | m | ponteiro para string | extra do draft, não estava no design |
| `endereço de T` | m | palavra | |
| `sequência de N Ts` | f | contígua, tamanho fixo | |
| registro | do artigo | layout com alinhamento real no x86 | |

Estouro dá a volta (`byte 250 + 10 = 4`, `int8 127 + 1 = -128`). Inteiro cabe em
inteiro de outra largura (truncado); real não cabe direto em inteiro (`como`).

## 5. Erros

Categorias (`ROTULO_ERRO`): léxico, morfologia, concordância, regência, sintaxe,
escopo, tipo, execução. Cada erro de compilação tem linha, coluna, mensagem na língua
da gramática e às vezes dica (inclusive "Faltou acento? Escreva ...").

Exemplos reais:

```
"o prova" não concorda: "prova" é feminino singular. Escreva "a prova".
Contração obrigatória: "de as" se escreve "das".
Em condição, o predicado vai no futuro do subjuntivo: "vir", não "ver".
"transferir" foi definido com a preposição "de" antes de a origem: transfiro QUANTIA de ORIGEM para DESTINO.
Antes do sujeito de um infinitivo não se contrai: escreva "o fato de a ...", não "o fato da ...".
"entre" indica simetria e só vale para operação comutativa.
"Escreve" está na 3ª pessoa. Lavra narra em 1ª pessoa: "escrevo".
```

Erros de execução: divisão e resto por zero, posição fora da sequência, acesso ao
apontado por nada, liberação dupla, uso de memória liberada, endereço de variável
fora de escopo (só interpretador), leitura antes de valor (só interpretador), função
que termina sem devolver, estouro de pilha, vazamento (aviso no fim).

## 6. EBNF de design (primeira conversa)

Mantido como referência; a implementação foi além em alguns pontos (complemento fixo,
predicado adjetivo, `entre X e Y`, `texto`, hex, `Nota:`/`--`).

```ebnf
texto        = { def_função | def_registro | período } ;
período      = oração { coord oração } "."
             | cabeça ":" ( oração { coord oração } "." | NL bloco ) ;
bloco        = INDENT período { período } DEDENT ;
coord        = "e" | "," ;
oração       = declaração | guarda | ajuste | escrita | leitura
             | retorno | chamada | memória | "paro" | "pulo" ;
declaração   = ( "seja" | "sejam" ) sujeito [ "," ADJ_FIXO "," ]
               predicado [ "," "valendo" expr ] ;
sujeito      = nome { ( "," | "e" ) nome } ;
predicado    = ART_INDEF tipo | TIPO_PLURAL ;
guarda       = "guardo" expr "em" alvo ;
ajuste       = "acrescento" expr "a" alvo | "subtraio" expr "de" alvo
             | "multiplico" alvo "por" expr | "divido" alvo "por" expr ;
escrita      = "escrevo" expr { "seguido de" expr } ;
leitura      = "leio" ( alvo | ART_INDEF tipo ) ;
retorno      = "devolvo" [ expr ] ;
chamada      = VERBO_1PS argumentos ;
memória      = "reservo" expr ( "bytes" | TIPO_PLURAL ) | "libero" alvo ;
cabeça       = "se" condição | "senão" [ "," "se" condição ]
             | "enquanto" condição | "repito" ( "sempre" | expr "vezes" )
             | "para" "cada" ( SÍMBOLO "de" expr "até" expr | NOME_SING [ "de" nome ] ) ;
condição     = cláusula { ( "e" | "ou" ) cláusula } ;
cláusula     = expr [ "não" ] ( "for" relação | VERBO_SUBJ argumentos ) ;
relação      = "maior que" expr | "menor que" expr | "igual a" expr
             | "diferente de" expr | "pelo menos" expr | "no máximo" expr | "nada" ;
expr         = termo { ( "mais" | "menos" ) termo } ;
termo        = fator { ( "vezes" | "dividido por" | "módulo" ) fator } ;
fator        = primário [ "como" tipo ] ;
primário     = LITERAL | SÍMBOLO | alvo | nominal
             | "o resultado" [ "de" INFINITIVO argumentos ]
             | "o fato de" expr ( "ser" relação | INFINITIVO argumentos )
             | "(" expr ")" ;
nominal      = ART_DEF OP_BIN ( "de" | "entre" ) expr "e" expr
             | ART_DEF OP_UN "de" expr
             | "o deslocamento de" expr "por" expr ( "à esquerda" | "à direita" ) ;
alvo         = nome | SÍMBOLO | ART_DEF NOME "de" alvo | ART_DEF ORDINAL "de" nome
             | "o item" expr "de" nome | "o endereço de" alvo | "o apontado por" expr ;
nome         = ART_DEF NOME ;
argumentos   = [ expr { "e" expr } ] { PREP expr } ;
def_função   = "para" [ param ] INFINITIVO [ "(" VERBO_1PS ")" ] parâmetros
               [ "," "sendo" tipagem { "e" tipagem } ]
               [ "," "devolvendo" ART_INDEF tipo ] ":" NL bloco ;
parâmetros   = [ param { "e" param } ] { PREP param } ;
param        = SÍMBOLO | nome ;
tipagem      = ( param { ( "," | "e" ) param } | "ambos" | "ambas" ) predicado ;
def_registro = ART_INDEF NOME "é um registro de" ":" NL
               INDENT campo { ";" NL campo } "." DEDENT ;
campo        = nome "," predicado ;
tipo         = ( "inteiro" | "natural" ) [ "de" NÚMERO "bits" ]
             | "real" [ "de" NÚMERO "bits" ] | "byte" | "lógico" | "caractere"
             | "endereço de" tipo | "sequência de" NÚMERO TIPO_PLURAL | NOME_REGISTRO ;
```

Verificações fora do EBNF: concordância, regência, escopo de "o resultado".

## 7. Mapeamento para máquina (do design)

```
Seja o contador um inteiro, valendo 0.  →  mov  qword [rbp-8], 0
Acrescento 1 ao contador.               →  add  qword [rbp-8], 1
Se o contador for menor que 10:         →  cmp  qword [rbp-8], 10 / jge .senão_1
Guardo 0 no apontado pelo cursor.       →  mov  rax, [rbp-16] / mov byte [rax], 0
Elevo 2 a 10 e escrevo o resultado.     →  call elevar / mov rdi, rax / call escrever
```

O gerador real comenta cada bloco de instruções com a frase original
(`# ── 5: Guardo a união ...`) e cada acesso à memória com o nome da variável.
