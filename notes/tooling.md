# Suporte de editor

## VS Code (`editors/vscode`)

Extensão `joaohenriquebarbosa.lavra`, instalada no VS Code do WSL em 2026-10-22 com
`code --install-extension lavra.vsix`.

- **Gramática TextMate** `source.lavra`, gerada por `syntaxes/build.py` a partir do
  léxico que o compilador exporta (`editors/lexicon.json`, de `lavrac lexicon --json`):
  as palavras vêm do léxico e o `build.py` só monta as regras; o JSON não se edita à mão.
  Palavra nova na língua entra em `src/lexicon.rs`, o JSON se regera
  (`LAVRA_BLESS=1 cargo test --lib lexicon`) e a gramática também (`npm test`).
  Escopos padrão para os temas pintarem sem configuração: `storage.type.declaration` (é/são), `storage.modifier.mutable`
  (começa), `keyword.other.mutation` (vira, cresce, ganha, perde, diminui),
  `keyword.control.*`, `entity.name.function` (cabeçalho, predicado, chamada em 1ª
  pessoa, verbo depois de "o resultado de", adjetivo depois de "for"),
  `variable.parameter`, `variable.other` (nome com artigo), `variable.language.result`
  (o resultado), `storage.modifier.borrowed/lent/maybe` (emprestado, confiado, talvez),
  `storage.type.primitive`, `entity.name.type` (registros, enumerações, casos),
  `support.function.nominal` (a soma de ...), `keyword.operator.*`, literais e comentários.
  O artigo leva o escopo da palavra que determina ("o tamanho", "o resultado", "o item");
  a contração leva o de preposição ("seguido do tamanho").
- Fronteira de palavra própria (`\b` do Oniguruma não serve para "é"); tudo sem
  diferenciar maiúsculas, porque a primeira palavra da oração vem maiúscula.
- O TextMate escolhe o casamento que começa antes na linha; por isso a regra de chamada
  do usuário (ancorada na margem) exclui explicitamente as palavras da língua e as
  continuações de linha.
- **Limite do léxico**: o verbo de predicado em `Se o saldo exceder o teto` e o
  complemento fixo (`os dias` em "contar os dias até") não se distinguem pela forma;
  vão precisar dos tokens semânticos do servidor de linguagem.
- **Configuração**: comentário `--`, pares, recuo depois de `:`, `Senão` desrecua, dobra
  por indentação (`offSide`), palavra com acento e hífen.

### Testes

- `npm test`: asserções de escopo (`tests/syntax/*.lav`, formato do
  `vscode-tmgrammar-test`, geradas por `scripts/make_syntax_tests.py`) e snapshots de
  todos os `samples/` (`tests/snap`, links para os samples).
- `node scripts/tokens.mjs arquivo.lav`: escopos de cada token, com o motor do VS Code.
- `node scripts/preview.mjs saída.html arquivos...`: renderiza com shiki nos temas
  Dark+ e Light+; a captura sai com `chromium --headless=new --screenshot`.

## Servidor de linguagem (`lavrac lsp`)

`src/lsp/` (feature `lsp`, padrão), síncrono sobre stdio com `lsp-server` e `lsp-types`.
Cada mudança reanalisa o documento inteiro com `lavra::analyze`, que não para no primeiro
erro; cada requisição roda isolada (`catch_unwind`), e um defeito vira erro só dela.

- **Posições**: UTF-8 quando o cliente aceita, UTF-16 nos outros (o VS Code), pelo
  `LineIndex` de `span.rs`.
- **Diagnósticos**: todos, com o código, o link para a explicação, a dica e as notas na
  mensagem, os rótulos como informação relacionada, `Unnecessary` nos avisos de não uso e
  no "Senão" que nunca acontece, e as correções em `data` (para as ações rápidas).
- **Tokens semânticos**: das tabelas da análise (funções, também nos predicados e depois de
  "o resultado de"; parâmetros com `borrowed` e `lent`; variáveis, `readonly` para o que "é";
  campos, tipos e casos; `declaration` e `modification`) e do vocabulário (tipos e casos
  citados). As palavras da língua ficam com a gramática TextMate. A contração conta um token.
- **Hover**: nome com artigo, fixo ou variável, o tipo por extenso, o modo de posse e a linha
  da declaração; verbo com a regência, o retorno e se pode falhar; palavra da língua com a
  documentação do léxico.
- **Navegação**: definição, referências (com leitura e escrita), destaque, símbolos do
  documento (funções com a regência, registros com campos, enumerações com casos,
  constantes) e do workspace (documentos abertos e `.lav` das pastas).
- **Testes**: `tests/lsp.rs` roda o servidor em processo (`Connection::memory()`).

## Próximos passos

Servidor de linguagem (`lavrac lsp`, reaproveitando léxico, parser, sema e posse):
diagnósticos ao digitar, tokens semânticos (verbos e tipos do programa), hover com
tipo e modo de posse, ir para a definição, completar verbos com a regência,
formatação.
