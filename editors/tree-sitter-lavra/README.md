# tree-sitter-lavra

A gramática [tree-sitter](https://tree-sitter.github.io) da Lavra, para o realce, o recuo,
as dobras e a estrutura nos editores que usam tree-sitter: Helix, Neovim e Zed (as
configurações estão em [`../helix`](../helix), [`../neovim`](../neovim) e [`../zed`](../zed)).

É rasa de propósito. Ela reconhece orações, blocos, cabeçalhos de função, definições de tipo,
comentários, textos, números, as palavras da língua pelo papel e os nomes com artigo. O
detalhe fino, como o que é verbo do programa, o parâmetro emprestado ou o campo de um
registro, fica com os tokens semânticos do servidor de linguagem (`lavrac lsp`), que sabe o
que a análise sabe.

## As palavras vêm do léxico

O `grammar.js` não tem lista de palavras: ele lê o `editors/lexicon.json`, que o compilador
exporta (`lavrac lexicon --json`, regravado por `LAVRA_BLESS=1 cargo test --lib lexicon`).

- Cada palavra vira um token, com um papel só. Quando o léxico dá mais de um papel à mesma
  forma, vale o primeiro da ordem em `ROLE_ORDER` ("a" é artigo antes de preposição).
- As locuções ("igual a", "maior que") se leem palavra por palavra; a palavra que não tem
  outro papel fica com o da locução.
- "Nota", as formas recusadas com dica e as reservadas não viram palavra da língua: podem ser
  nomes do programa ("a nota").
- As construções que começam por uma palavra da língua ("Se", "Para", "Uso", "Uma") usam a
  palavra direto, com um nó próprio para o realce (`conditional_keyword`, `function_keyword`).
- As palavras não distinguem maiúsculas, como no compilador: a primeira palavra da oração vem
  maiúscula.

Palavra nova no léxico pede `npm run generate`, que regera `src/` e as cópias das queries; o
CI confere que o gerado está em dia.

## O scanner (`src/scanner.c`)

O recuo e o fim da linha lógica não cabem numa gramática livre de contexto; o scanner externo
segue as regras do léxico do compilador (`src/lexer.rs`):

- a linha só termina a linha lógica se acabar em ".", ":" ou ";"; senão a oração continua na
  linha de baixo, com recuo livre. O NEWLINE só sai logo depois desses sinais, no fim da linha;
- o bloco depois de ":" abre com INDENT e fecha com DEDENT, pela coluna da próxima linha de
  código; o tab vale quatro espaços, e linhas em branco e comentários não contam;
- o comentário é a linha que começa com "Nota:" (sem distinguir maiúsculas) ou com "--", e o
  "--" no meio da linha corta o resto dela. O comentário alinhado com o bloco fica dentro
  dele, e o que já está no recuo de fora fecha o bloco antes;
- o conteúdo dos textos também é do scanner, para "--" e "Nota:" dentro deles serem texto.

Os tokens de recuo e de fim de linha têm largura zero e ficam no fim da linha que os provoca.

## As queries

As de `queries/` usam os nomes de captura do Neovim (os padrão do tree-sitter): `highlights`,
`locals`, `folds`, `indents`, `tags` e `textobjects`. Os três editores resolvem do mesmo jeito
dois padrões sobre o mesmo trecho (vence o que vem depois), então as cópias de cada editor só
trocam os nomes de captura. Elas saem de `scripts/sync-queries.js`; não se editam à mão. O que
é de um editor só fica na pasta dele: o recuo do Helix e do Zed, os pares e o esboço do Zed.

## Gerar e testar

Com o Node 22 ou mais novo (o `tree-sitter-cli` fica preso no `package-lock.json`):

```bash
npm ci
npm run generate        # src/ a partir do grammar.js e do léxico, e as cópias das queries
npm test                # tudo abaixo
npm run test:corpus     # test/corpus/*.txt e o realce de test/highlight/realce.lav
npm run test:queries    # as cópias em dia, e toda query de cada editor compila
npm run test:drift      # o teste de deriva
```

- **Corpus** (`test/corpus/`): trechos com a árvore esperada, campos inclusive.
- **Realce** (`test/highlight/realce.lav`): asserções de captura sobre um programa inteiro.
- **Deriva** (`scripts/drift.js`): todo programa certo do repositório (`samples/`,
  `tests/programs/ok/`, os de `test/programs/` e as fixtures da integração do VS Code) parseia
  sem nó ERROR nem MISSING, toda linha de comentário vira nó `comment` e, no corpus formatado,
  cada oração está na coluna do bloco em que a gramática a pôs. Os programas errados
  (`tests/programs/err/`) não derrubam o parser.
- **Programas da gramática** (`test/programs/`): Lavra válida com o que o corpus formatado não
  tem (tabs, CRLF, sem quebra final, maiúsculas, enjambement); o `cargo test` do compilador
  confere que ela compila (`tests/editor_programs.rs`).

Nos editores de verdade, cada um com a configuração do repositório:

```bash
scripts/helix-health.sh     # Helix 25.07+: compila a gramática, hx --health lavra, abre um programa
scripts/neovim-check.sh     # Neovim 0.11+: parser, queries, ftplugin e servidor, sobre o corpus
```

O parser sai com a ABI 14, que os três editores carregam.
