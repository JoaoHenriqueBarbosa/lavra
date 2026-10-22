# Arquitetura do draft

Tudo em JavaScript puro (sem dependências, sem build), mais um runtime em C. Roda em
Node (CLI) e no navegador (editor). Identificadores do código estão em português
(`Analisador`, `Maquina`, `compilar`, `falha`...).

## Arquivos

```
draft/
  lavra.zip              pacote original da primeira conversa (2026-10-21 21:02)
  lavra/                 conteúdo extraído do zip
    lavra.js             2308 linhas: léxico, morfologia, parser + verificações, interpretador, visualizações
    x86.js               837 linhas: gerador de assembly x86-64 (Intel, GNU as, System V)
    runtime.c            257 linhas: E/S no formato Lavra, reservas, erros de execução
    lavrac.js            38 linhas: CLI (interpreta, -s gera .s, -c gera executável via gcc)
    lavra.html           3837 linhas: editor autocontido (embute lavra.js e x86.js)
    LEIAME.md
    exemplos/            completo, lista2, media, regencia, script, sequencias, tropeco
  lavra.js               versão MAIS NOVA que a do zip (patch de 1 linha, ver draft-status.md)
  lavra.html             idem, com o mesmo patch embutido
  lavra-dos-dias.lav     poemas da segunda conversa
  a-partilha.lav
  catilinaria.lav
```

Os arquivos soltos na raiz do `draft/` são a versão final; `draft/lavra/` é o zip
anterior ao patch.

## Pipeline (`lavra.js`)

`compilar(fonte)` → `{ ok, prog, toks }` ou `{ ok: false, erro, toks }`.

1. **`lexar`**: normaliza fonte, quebra em linhas, ignora comentários, calcula
   INDENT/DEDENT com pilha de recuos, emite NL só depois de `.`, `:` ou `;` (senão a
   linha continua). Depois: `fundir(LOC_PROTEGIDAS)` → `desfazerContracoes` →
   `fundir(LOCUCOES)`. Tokens: `{ t: 'w'|'n'|'s'|'c'|'p'|INDENT|DEDENT|NL|EOF, v, o
   (original), line, col, contr, parte, loc, bol }`.
2. **`Analisador`** (classe única, parser descendente recursivo que já faz as
   verificações durante o parse, não numa passada separada):
   - `preVarredura()`: acha inícios de período no nível 0; registros (`Um X é ...`)
     e cabeçalhos de função (`Para ...`, exceto `Para cada`). Processa nomes de
     registros, depois campos, calcula tamanhos em **células** (detecta recursão
     infinita), depois todos os cabeçalhos (para permitir uso antes da definição).
   - Tabelas: `registros`/`registrosPl`, `porForma` (1ª pessoa → função),
     `porInf`, `predSubj` (forma subjuntiva → predicado), `predInf`, `predAdj`.
   - `programa()` percorre o texto pulando registros já lidos e chamando
     `corpoFuncao` nos inícios de função; o resto vira `corpo` do nível de fora.
   - Escopo: cadeia de `{ vars: Map, pai }`; cada declaração ganha `id` único.
   - `this.res` rastreia o tipo de "o resultado" dentro do período.
   - `anotar(tk, classe, info)` grava em `tk.ana` a análise morfológica de cada
     token, usada pelo realce do editor e pela aba Morfologia.
   - Erros: `falha(tipo, msg, tok, dica)` lança `ErroLavra`; `esperado()` detecta
     "faltou ponto final" e sugere acento (`dicaAcento`).
   - AST: nós com `k` (`decl`, `guarda`, `ajuste`, `escrita`, `leitura`, `retorno`,
     `chamada`, `reserva`, `libera`, `paro`, `pulo`, `se`, `enquanto`, `repete`,
     `faixa`, `cada`, `periodo`; expressões `num`, `texto`, `car`, `logico`, `nada`,
     `var`, `campo`, `indice`, `apontado`, `endereco`, `bin`, `un`, `conv`,
     `anafora`, `chamada`, `fato`; condições `e`, `ou`, `cmp`, `pred`), todos com
     `tipo` e `tok`.
3. **`ligarRetornos`**: liga cada `Devolvo` à função dona (o interpretador precisa do
   tipo de retorno para coagir).

## Interpretador (`Maquina`)

- Memória de **células** (não bytes): `mem[]` com valores JS (BigInt para inteiros,
  Number para reais) e `est[]` com estado por célula: 1 pilha, 2 monte, 3 liberada,
  4 fora de escopo. Pilha a partir de `0x100`, monte a partir de `0x100000`.
- Tamanho de tipo = número de células (`tam`), não bytes; registros e sequências
  ocupam células contíguas.
- Execução por **geradores** (`function*`): `yield { k: 'ler' }` pede entrada,
  `yield { k: 'tique' }` a cada 2000 passos (o editor usa para não travar a aba e
  para o botão Parar). `executarSincrono(prog, entradas)` serve para testes.
- Inteiros com `BigInt.asIntN/asUintN` (estouro dá a volta), real 32 com
  `Math.fround`.
- Detecta: leitura antes de valor, uso após liberar, liberação dupla, escopo morto,
  índice fora, divisão por zero, recursão > 2000, vazamento no fim (aviso).

## Gerador x86-64 (`x86.js`)

- `gerarX86(prog, fonte)` → `{ ok, asm }` ou `{ ok: false, erro }`.
- Layout **real em bytes** com alinhamento (`sizeOf`, `alignOf`, `layoutRec`
  calcula `offB` por campo), independente das células do interpretador.
- Quadro por função planejado antes (`planeja`): slots de variáveis, ocultos de
  laço, slot de "o resultado". Globais vão para `.bss`.
- Valores em `rax` (inteiros, ponteiros, texto) ou `xmm0` (reais); compostos são
  passados por endereço e copiados com `memcpy`. Até 6 argumentos inteiros e 8 reais
  (sem passagem pela pilha).
- Comenta cada bloco de instruções com a linha-fonte (`# ── N: frase`) e cada acesso
  com o nome da variável.
- Chamadas ao runtime via `@PLT`, mantendo a pilha alinhada em 16.

## Runtime (`runtime.c`)

- `lavra_inicia`/`lavra_termina` (aviso de vazamento), `lavra_falha(linha, msg)`.
- Saída: `lavra_escreve_{i,u,r,b,c,t,p}`, `lavra_escreve_bloco` (descritor textual
  do tipo, ex. `{x@0=r64,y@8=r64}`), `lavra_nl`. Real com vírgula, até 12 dígitos.
- Texto: `lavra_txt_*` (conversões, sempre `malloc`, nunca liberadas),
  `lavra_concatena`, `lavra_tamanho_texto` (conta code points UTF-8).
- Entrada: `lavra_le_{i,r,c,b,t}` com `LAVRA_ECO` no ambiente para ecoar a entrada.
- Memória: `lavra_reserva`/`lavra_libera` com listas de blocos vivos e mortos
  (detecta liberação dupla e liberação de não-início).

## Editor (`lavra.html`)

- Um arquivo só: CSS, a gramática de referência em HTML, e três `<script>`:
  `lavra.js` inteiro, `x86.js` inteiro, e o script da interface.
- Textarea transparente sobre um `<pre>` de realce; cada token pintado pela classe
  gramatical (verbo vermelho, nome azul, artigo cinza, preposição ocre, tipo verde).
  Tema claro de papel almaço (pauta azul, margem vermelha), escuro de lousa.
- Compila a cada tecla; erro sublinha a palavra e mostra rótulo, linha e dica.
- Abas: Saída (com caixa de entrada inline quando o programa lê), Morfologia (linha
  do cursor ou programa inteiro), Árvore, x86-64 (com botão Copiar), Gramática.
- Exemplos embutidos num `const EXEMPLOS` (JSON), seletor no topo.

## CLI (`lavrac.js`)

```
node lavrac.js prog.lav       interpreta (stdin como entrada, sem eco)
node lavrac.js -s prog.lav    escreve prog.s ao lado do .lav
node lavrac.js -c prog.lav    gera prog.s e o executável prog ao lado do .lav (gcc + runtime.c -lm)
```

Atenção: `-s` e `-c` escrevem artefatos ao lado do `.lav`. Para não sujar o draft,
rodar numa cópia (ex. `/tmp/lavra-run`).
