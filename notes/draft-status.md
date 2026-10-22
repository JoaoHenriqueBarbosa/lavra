# Estado do draft (verificado em 2026-10-22)

## O que foi rodado

Numa cópia em `/tmp/lavra-run` (zip extraído + `lavra.js` da raiz), com Node 26 e
gcc 14 no host:

| Programa | Interpretador | Nativo x86-64 |
|---|---|---|
| `script.lav` | ok | idêntico |
| `sequencias.lav` | ok | idêntico |
| `regencia.lav` | ok | idêntico |
| `lista2.lav` | ok (385) | idêntico |
| `completo.lav` | ok | idêntico (conferido o final) |
| `tropeco.lav` | erro de concordância esperado ("o prova") | n/a |
| `lavra-dos-dias.lav` | ok (13/05/1990 a 22/10/2026: domingo, 13311 dias) | idêntico |
| `a-partilha.lav` | ok (13x21: 7 dias de corte, 273 lotes) | compila |
| `catilinaria.lav` | ok | compila |

## O patch que só existe na raiz

`draft/lavra.js` e `draft/lavra.html` diferem da versão do zip em uma linha, em
`ehParamInicio`:

```js
// zip
if (ARTIGOS[tk.v] && ARTIGOS[tk.v].def) {
// raiz
if (ARTIGOS[tk.v] && ARTIGOS[tk.v].def && tk.parte !== 1) {
```

Sem ele, o cabeçalho `Para responder ao cônsul com a defesa` falha: o `a` da
contração `ao` (parte 1, que é preposição) era lido como artigo e o parser achava que
começava um objeto. Com o zip, `catilinaria.lav` dá erro na linha 74. A causa de fundo
é que o token da contração desfeita reaproveita o mesmo valor (`a`) para preposição e
artigo, e cada ponto do parser precisa lembrar de olhar `parte`.

## Limitações declaradas pelo próprio draft

- O interpretador simula memória em **células**, não bytes; o x86 usa bytes. Dois
  modelos de layout diferentes (`tam` vs `sizeOf`).
- O binário nativo **não acusa leitura de variável sem valor** (confirmado: escreve
  lixo, `109745916628424`; o interpretador dá erro de execução).
- O gerador x86 **não devolve registros nem sequências por valor** (só endereço).
- Até 6 argumentos inteiros e 8 reais por função no x86 (sem argumentos na pilha).

## Problemas encontrados

- **Ambiguidade nominal + infixa**: `a diferença de 10 e 3 mais 1` dá 6, ou seja
  10 - (3 + 1). O segundo argumento de uma operação nominal é uma `expr` completa e
  engole o infixo. O design dizia que a forma nominal "nunca é ambígua", mas a mistura
  com a infixa reabre a ambiguidade. Foi exatamente o bug do programa *ab urbe
  condita* (689 em vez de 691).
- **Cabeçalho "substantivo + símbolo"** não existe: `Para ligar o bit n` dá erro de
  sintaxe ("Esperava dois-pontos ... encontrei n"). Frases de hardware (`o pino 13`,
  `a porta 2`) dependem disso.
- **Real é ponto flutuante**: `o fato de 0,1 mais 0,2 ser igual a 0,3` é falso; só
  "parece" 0,3 porque a impressão arredonda a 12 dígitos. Sem tipo decimal, sem datas.
- **CLI do interpretador lê o stdin inteiro antes de rodar** (`readFileSync(0)`),
  então trava esperando EOF mesmo em programa sem leitura, e não ecoa a entrada (os
  prompts saem colados). O nativo ecoa com `LAVRA_ECO`.
- **Código duplicado**: `lavra.html` embute cópias de `lavra.js` e `x86.js`; a
  sincronia é manual (o patch teve que ser aplicado nos dois).
- **Textos no nativo vazam**: toda conversão e concatenação faz `malloc` e nunca
  libera.
- **Restos mortos**: `isPredicadoAdiante()` sempre devolve `false`; `funcDe` cai em
  `this.funcAtualExec`, que nunca é definido (funciona porque `ligarRetornos` sempre
  preenche `o.f`).
- **Parser e verificação no mesmo passo**: o `Analisador` faz parse, escopo, tipos e
  gramática juntos, com backtracking pontual (`try/catch` em campo de registro). Não
  há AST "crua" separada da verificada.
- **Identificadores em português** no código do compilador (contra a regra global do
  user para código novo; ver `CLAUDE.md`).
- Sem testes automatizados: a verificação foi feita à mão comparando saída do
  interpretador com a do nativo.
- `a`, `e`, `o` não podem ser símbolos (por isso `x` e `y` no lugar de `a` e `b`).

## O que o draft não tem e foi citado como próximo passo

- Aba de memória e modo passo a passo no editor.
- Extensão do cabeçalho para `o pino 13`.
- Backend que gere C (abre AVR, ESP32, RP2040, WebAssembly).
- Segundo andar da gramática: orações subordinadas como funções anônimas ("guardo em x
  o que sobrar de dividir ...") e voz passiva.
- Pretérito, enumerações com concordância, backends RISC-V/ARM64.
- Programa *ab urbe condita* (só existe descrito na conversa).
