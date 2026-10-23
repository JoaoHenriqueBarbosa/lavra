-- O servidor de linguagem da Lavra (`lavrac lsp`) no Neovim 0.11 ou mais novo. O plugin o liga
-- com vim.lsp.enable('lavrac') quando acha o comando; o que você puser em
-- vim.lsp.config('lavrac', { ... }) se junta a isto e vale por cima, em qualquer ordem.
--
-- Os inlay hints (o tipo das declarações sem aposto, o modo dos argumentos e o tipo de
-- "o resultado") começam todos ligados. Para desligar um, por exemplo o tipo:
--
--   vim.lsp.config('lavrac', { init_options = { inlayHints = { types = false } } })
--
-- e, para vê-los, vim.lsp.inlay_hint.enable().

---@type vim.lsp.Config
return {
  cmd = { 'lavrac', 'lsp' },
  filetypes = { 'lavra' },
  root_markers = { '.git' },
  -- A definição de um verbo de biblioteca vai para "lavra-library:/textos.lav", que o plugin
  -- abre com o texto pedido ao servidor (o pedido lavra/library).
  init_options = { libraryDocuments = true },
}
