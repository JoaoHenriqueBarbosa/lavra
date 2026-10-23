-- A Lavra no Neovim (0.11 ou mais novo): o parser no nvim-treesitter (`:TSInstall lavra`), o
-- servidor de linguagem (lsp/lavrac.lua, ligado aqui quando o comando existe), o texto das
-- bibliotecas na definição dos verbos delas e o depurador do nvim-dap, com o mesmo lavrac.

if vim.g.loaded_lavra then
  return
end
vim.g.loaded_lavra = true

local group = vim.api.nvim_create_augroup('lavra', { clear = true })

-- O parser vem do repositório da Lavra, da pasta da gramática; as queries vêm deste plugin.
vim.api.nvim_create_autocmd('User', {
  group = group,
  pattern = 'TSUpdate',
  callback = function()
    require('nvim-treesitter.parsers').lavra = {
      install_info = {
        url = 'https://github.com/JoaoHenriqueBarbosa/lavra',
        location = 'editors/tree-sitter-lavra',
      },
    }
  end,
})

-- O nome no ponto em que entrega o valor (o modificador `moved` dos tokens semânticos): só
-- itálico, com a cor que o nome já tem. É um padrão: o tema ou você podem trocá-lo.
local function moved_highlight()
  vim.api.nvim_set_hl(0, '@lsp.mod.moved.lavra', { italic = true, default = true })
end
moved_highlight()
vim.api.nvim_create_autocmd('ColorScheme', { group = group, callback = moved_highlight })

-- O texto de uma biblioteca ("lavra-library:/textos.lav"), aberto pela definição de um verbo
-- dela: vem do servidor (lavra/library) e fica só de leitura, com o realce da Lavra e sem tipo
-- de arquivo, para o servidor não o analisar como programa.
vim.api.nvim_create_autocmd('BufReadCmd', {
  group = group,
  pattern = 'lavra-library:/*',
  callback = function(args)
    local buf = args.buf
    local uri = vim.api.nvim_buf_get_name(buf)
    local client = vim.lsp.get_clients({ name = 'lavrac' })[1]
    local response = client and client:request_sync('lavra/library', { uri = uri }, 5000, buf)
    local text = response and response.result and response.result.text
    if type(text) ~= 'string' then
      local why = not client and 'o servidor de linguagem não está rodando'
        or (response and response.err and response.err.message)
        or 'o servidor não respondeu'
      vim.notify(('Lavra: não abri %s: %s.'):format(uri, why), vim.log.levels.ERROR)
      return
    end
    local lines = vim.split(text, '\n', { plain = true })
    if lines[#lines] == '' then
      table.remove(lines)
    end
    vim.bo[buf].modifiable = true
    vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
    vim.bo[buf].buftype = 'nofile'
    vim.bo[buf].modified = false
    vim.bo[buf].modifiable = false
    vim.bo[buf].readonly = true
    pcall(vim.treesitter.start, buf, 'lavra')
  end,
})

if not (vim.lsp.config and vim.lsp.enable) then
  return
end

-- O comando do servidor, com o que você configurou em vim.lsp.config('lavrac', ...).
local config = vim.lsp.config.lavrac
local lavrac = config and type(config.cmd) == 'table' and config.cmd[1]
if not lavrac or vim.fn.executable(lavrac) ~= 1 then
  return
end
vim.lsp.enable('lavrac')

-- O adaptador de depuração (`lavrac dap`), para quem usa o nvim-dap.
vim.api.nvim_create_autocmd('FileType', {
  group = group,
  pattern = 'lavra',
  once = true,
  callback = function()
    local ok, dap = pcall(require, 'dap')
    if not ok then
      return
    end
    dap.adapters.lavra = dap.adapters.lavra or { type = 'executable', command = lavrac, args = { 'dap' } }
    dap.configurations.lavra = dap.configurations.lavra
      or {
        { type = 'lavra', request = 'launch', name = 'Depurar o arquivo', program = '${file}' },
        {
          type = 'lavra',
          request = 'launch',
          name = 'Depurar o arquivo, com a entrada de um arquivo',
          program = '${file}',
          stdin = function()
            return vim.fn.input('Entrada: ', '', 'file')
          end,
        },
      }
  end,
})
