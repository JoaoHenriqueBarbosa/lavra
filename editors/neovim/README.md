# A Lavra no Neovim

Um plugin para o Neovim 0.11 ou mais novo: o tipo de arquivo `lavra`, o realce, as dobras e
o recuo pela gramática tree-sitter, o servidor de linguagem e o depurador pelo próprio
`lavrac`, e o texto das bibliotecas na definição dos verbos delas.

## Instalar

1. Instale o `lavrac` no PATH, pela [página da última versão](https://github.com/JoaoHenriqueBarbosa/lavra/releases/latest).
2. Ponha esta pasta no runtimepath. Com uma cópia do repositório, pelo lazy.nvim:

   ```lua
   { dir = '/caminho/para/lavra/editors/neovim' }
   ```

   ou, sem gerenciador, no começo do `init.lua`:

   ```lua
   vim.opt.rtp:append('/caminho/para/lavra/editors/neovim')
   ```

3. Instale o parser, de um dos dois jeitos:
   - com o [nvim-treesitter](https://github.com/nvim-treesitter/nvim-treesitter) (branch
     `main`, que pede o Neovim 0.12 e o tree-sitter CLI 0.26.1 ou mais novos), que o plugin
     já conhece: `:TSInstall lavra`;
   - sem ele, compilando com o compilador de C do sistema:

     ```bash
     cd /caminho/para/lavra/editors/tree-sitter-lavra
     mkdir -p ~/.local/share/nvim/site/parser
     cc -O2 -shared -fPIC -I src src/parser.c src/scanner.c -o ~/.local/share/nvim/site/parser/lavra.so
     ```

As queries (realce, escopos, dobras, recuo e objetos de texto) vêm do plugin, em
`queries/lavra`; o ftplugin liga o realce sozinho. As dobras pela árvore:

```lua
vim.api.nvim_create_autocmd('FileType', {
  pattern = 'lavra',
  callback = function()
    vim.wo.foldmethod = 'expr'
    vim.wo.foldexpr = 'v:lua.vim.treesitter.foldexpr()'
  end,
})
```

O recuo pela árvore vem do nvim-treesitter, quando ele está instalado.

## O servidor de linguagem

O plugin liga o servidor (`vim.lsp.enable('lavrac')`) quando acha o comando. A configuração
está em [`lsp/lavrac.lua`](lsp/lavrac.lua), e o que você puser em `vim.lsp.config` se junta a
ela e vale por cima:

```lua
vim.lsp.config('lavrac', {
  cmd = { '/caminho/do/lavrac', 'lsp' },
  -- Os inlay hints começam todos ligados: o tipo das declarações sem aposto (types), o
  -- modo dos argumentos (modes) e o tipo de "o resultado" (result).
  init_options = { inlayHints = { types = false } },
})
vim.lsp.inlay_hint.enable()   -- o Neovim não mostra os inlay hints sem isto
```

- **Tokens semânticos.** O Neovim os aplica sozinho. O nome no ponto em que entrega o valor
  (o modificador `moved`) sai em itálico, com a cor que já tem, pelo grupo
  `@lsp.mod.moved.lavra`, que o plugin define como padrão; para trocar:
  `vim.api.nvim_set_hl(0, '@lsp.mod.moved.lavra', { italic = true, underline = true })`.
- **O texto das bibliotecas.** A definição de um verbo de biblioteca ("Divido", de
  `Uso a biblioteca de textos.`) abre `lavra-library:/textos.lav`: o texto da biblioteca,
  pedido ao servidor, só de leitura e com realce, fora da análise.

## O depurador

Com o [nvim-dap](https://github.com/mfussenegger/nvim-dap), o plugin registra o adaptador
(`lavrac dap`, com o mesmo `lavrac` do servidor) e duas configurações: "Depurar o arquivo" e
"Depurar o arquivo, com a entrada de um arquivo".

## Conferir

`editors/tree-sitter-lavra/scripts/neovim-check.sh` roda o Neovim headless, sem a sua
configuração: compila o parser, abre todos os programas certos do repositório e confere o tipo
de arquivo, o ftplugin, as queries, o realce sem erro de sintaxe e a dobra; com o `lavrac` no
PATH, também o diagnóstico, o `moved`, os inlay hints configurados e a definição de um verbo
de biblioteca.
