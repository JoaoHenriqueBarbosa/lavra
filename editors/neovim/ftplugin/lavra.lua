-- A Lavra no buffer: comentários com "--", recuo de quatro espaços e, quando o parser da
-- Lavra está instalado, o realce do tree-sitter (com o recuo do nvim-treesitter, se ele
-- estiver presente).

vim.bo.commentstring = '-- %s'
vim.bo.comments = ':--,:Nota:'
vim.bo.expandtab = true
vim.bo.shiftwidth = 4
vim.bo.softtabstop = 4
vim.bo.tabstop = 4

local undo = 'setlocal commentstring< comments< expandtab< shiftwidth< softtabstop< tabstop<'

if pcall(vim.treesitter.start) then
  local ok, nvim_treesitter = pcall(require, 'nvim-treesitter')
  if ok and nvim_treesitter.indentexpr then
    vim.bo.indentexpr = "v:lua.require'nvim-treesitter'.indentexpr()"
    undo = undo .. ' indentexpr<'
  end
end

vim.b.undo_ftplugin = (vim.b.undo_ftplugin and vim.b.undo_ftplugin .. ' | ' or '') .. undo
