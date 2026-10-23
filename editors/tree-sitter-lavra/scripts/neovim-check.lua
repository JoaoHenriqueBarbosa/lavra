-- Confere a Lavra no Neovim, headless e sem plugins de terceiros: o tipo de arquivo, o
-- ftplugin, o parser e as queries do plugin de editors/neovim, sobre todos os programas certos
-- do corpus, e a dobra pelo tree-sitter. Com o `lavrac` no PATH, também o servidor de
-- linguagem, que tem de publicar o diagnóstico de um programa errado. Quem roda este arquivo
-- é o scripts/neovim-check.sh, que compila o parser e passa as pastas pelo ambiente.

local root = assert(os.getenv('LAVRA_ROOT'), 'falta a variável LAVRA_ROOT')
local grammar = assert(os.getenv('LAVRA_GRAMMAR'), 'falta a variável LAVRA_GRAMMAR')
local site = assert(os.getenv('LAVRA_SITE'), 'falta a variável LAVRA_SITE')

local problems = {}
local function problem(text)
  table.insert(problems, text)
end

vim.opt.rtp:prepend(site)
vim.opt.rtp:prepend(root .. '/editors/neovim')
vim.cmd('runtime! plugin/lavra.lua')
vim.cmd('runtime! ftdetect/*.lua')
vim.cmd('filetype plugin on')

-- Como o usuário faria: desligar um tipo de inlay hint, por cima da configuração do plugin
-- (lsp/lavrac.lua), que tem de continuar valendo (o libraryDocuments).
local lavrac = vim.fn.executable('lavrac') == 1
if lavrac then
  vim.lsp.config('lavrac', { init_options = { inlayHints = { modes = false } } })
end

local loaded = {}
for _, name in ipairs({ 'highlights', 'locals', 'folds', 'indents', 'textobjects' }) do
  local ok, query = pcall(vim.treesitter.query.get, 'lavra', name)
  if ok and query then
    table.insert(loaded, ('%s (%d capturas)'):format(name, #query.captures))
  else
    problem(('a query %s não carregou: %s'):format(name, tostring(query)))
  end
end

local files = {}
for _, dir in ipairs({ root .. '/samples', root .. '/tests/programs/ok', grammar .. '/test/programs' }) do
  vim.list_extend(files, vim.fn.glob(dir .. '/*.lav', false, true))
end

local found, highlights = pcall(vim.treesitter.query.get, 'lavra', 'highlights')
if not found then
  highlights = nil
end
local captures = 0
for _, file in ipairs(files) do
  vim.cmd('edit ' .. vim.fn.fnameescape(file))
  local buf = vim.api.nvim_get_current_buf()
  local name = file:sub(#root + 2)
  local parser = vim.treesitter.get_parser(buf, 'lavra', { error = false })
  if vim.bo[buf].filetype ~= 'lavra' then
    problem(('%s: o tipo de arquivo é "%s"'):format(name, vim.bo[buf].filetype))
  elseif not vim.treesitter.highlighter.active[buf] then
    problem(name .. ': o ftplugin não ligou o realce do tree-sitter')
  elseif not parser then
    problem(name .. ': sem parser')
  else
    local tree = parser:parse()[1]:root()
    if tree:has_error() then
      problem(name .. ': erro de sintaxe na árvore')
    end
    if highlights then
      for _ in highlights:iter_captures(tree, buf) do
        captures = captures + 1
      end
    end
  end
end

-- O ftplugin e a dobra: o bloco de "Para começar:" dobra a partir do cabeçalho.
vim.cmd('edit ' .. vim.fn.fnameescape(root .. '/samples/catilinaria.lav'))
local options = ('commentstring=%s shiftwidth=%d expandtab=%s'):format(vim.bo.commentstring, vim.bo.shiftwidth, tostring(vim.bo.expandtab))
if options ~= 'commentstring=-- %s shiftwidth=4 expandtab=true' then
  problem('as opções do ftplugin estão erradas: ' .. options)
end
local header = vim.fn.search('^\\cpara começar:', 'nw')
local fold = ''
vim.wait(2000, function()
  fold = vim.treesitter.foldexpr(header)
  return fold:sub(1, 1) == '>'
end)
if fold:sub(1, 1) ~= '>' then
  problem(('a dobra não começa no cabeçalho da linha %d: foldexpr dá "%s"'):format(header, fold))
end

-- O servidor de linguagem, pelo plugin.
local server = { 'sem o lavrac no PATH, o servidor de linguagem não foi conferido' }
if lavrac then
  server = {}
  local fixture = root .. '/editors/vscode/tests/integration/fixture/'
  local function open(file)
    vim.cmd('edit ' .. vim.fn.fnameescape(file))
    local buf = vim.api.nvim_get_current_buf()
    local attached = vim.wait(10000, function()
      return #vim.lsp.get_clients({ bufnr = buf, name = 'lavrac' }) > 0
    end, 50)
    if not attached then
      problem(file:sub(#root + 2) .. ': o servidor de linguagem não se ligou ao buffer')
    end
    return buf, vim.lsp.get_clients({ bufnr = buf, name = 'lavrac' })[1]
  end

  -- O diagnóstico de um programa errado.
  local wrong = vim.fn.glob(root .. '/tests/programs/err/*.lav', false, true)[1]
  local buf = open(wrong)
  local published = vim.wait(15000, function()
    return #vim.diagnostic.get(buf, { severity = vim.diagnostic.severity.ERROR }) > 0
  end, 100)
  if published then
    local first = vim.diagnostic.get(buf, { severity = vim.diagnostic.severity.ERROR })[1]
    table.insert(server, ('diagnóstico em %s: %s %s'):format(wrong:sub(#root + 2), tostring(first.code), first.message:match('^[^\n]*')))
  else
    problem('o servidor de linguagem não publicou o diagnóstico de ' .. wrong:sub(#root + 2))
  end

  -- O nome que entrega o valor ("Arquivo as cartas.", com a lista passada por posse) chega com o
  -- modificador moved, e o plugin o põe em itálico.
  buf = open(fixture .. 'posse.lav')
  local row = vim.fn.search('^\\s*Arquivo as cartas', 'nw') - 1
  local col = vim.api.nvim_buf_get_lines(buf, row, row + 1, false)[1]:find('cartas') - 1
  local moved = vim.wait(10000, function()
    for _, token in ipairs(vim.lsp.semantic_tokens.get_at_pos(buf, row, col) or {}) do
      if token.modifiers and token.modifiers.moved then
        return true
      end
    end
    return false
  end, 100)
  local italic = vim.api.nvim_get_hl(0, { name = '@lsp.mod.moved.lavra' }).italic == true
  if moved and italic then
    table.insert(server, ('"cartas" na linha %d com o modificador moved, e @lsp.mod.moved.lavra em itálico'):format(row + 1))
  else
    problem(('moved em "cartas" na linha %d: token %s, itálico %s'):format(row + 1, tostring(moved), tostring(italic)))
  end

  -- Os inlay hints com a configuração do usuário: o tipo da declaração sim, o modo do
  -- argumento ("confiado:") não.
  local client
  buf, client = open(fixture .. 'conta.lav')
  local lines = vim.api.nvim_buf_line_count(buf)
  local hints = client
      and client:request_sync('textDocument/inlayHint', {
        textDocument = { uri = vim.uri_from_bufnr(buf) },
        range = { start = { line = 0, character = 0 }, ['end'] = { line = lines, character = 0 } },
      }, 5000, buf)
    or nil
  local labels = {}
  for _, hint in ipairs(hints and hints.result or {}) do
    local label = hint.label
    if type(label) == 'table' then
      label = table.concat(vim.tbl_map(function(part)
        return part.value
      end, label))
    end
    table.insert(labels, vim.trim(label))
  end
  local joined = table.concat(labels, ' | ')
  if joined:find('inteiro') and not joined:find('confiado') then
    table.insert(server, 'inlay hints com os modos desligados: ' .. joined)
  else
    problem('inlay hints com os modos desligados: ' .. (joined ~= '' and joined or 'nenhum'))
  end

  -- A definição de um verbo de biblioteca abre o texto dela, só de leitura e sem servidor.
  open(fixture .. 'textos.lav')
  row = vim.fn.search('^\\s*Divido', 'nw')
  vim.api.nvim_win_set_cursor(0, { row, vim.fn.indent(row) })
  vim.lsp.buf.definition()
  local opened = vim.wait(10000, function()
    return vim.api.nvim_buf_get_name(0):match('^lavra%-library:') ~= nil
  end, 50)
  if opened then
    local library = vim.api.nvim_get_current_buf()
    local line = vim.api.nvim_get_current_line()
    local state = {
      ('só leitura %s'):format(tostring(not vim.bo[library].modifiable and vim.bo[library].readonly)),
      ('realce %s'):format(tostring(vim.treesitter.highlighter.active[library] ~= nil)),
      ('servidores ligados %d'):format(#vim.lsp.get_clients({ bufnr = library })),
    }
    local good = not vim.bo[library].modifiable
      and vim.treesitter.highlighter.active[library] ~= nil
      and #vim.lsp.get_clients({ bufnr = library }) == 0
      and line:find('dividir') ~= nil
    local summary = ('%s, linha %d: "%s" (%s)'):format(vim.api.nvim_buf_get_name(library), vim.fn.line('.'), line, table.concat(state, ', '))
    if good then
      table.insert(server, 'definição de "Divido": ' .. summary)
    else
      problem('definição de "Divido": ' .. summary)
    end
  else
    problem('a definição de "Divido" não abriu o texto da biblioteca; o buffer é ' .. vim.api.nvim_buf_get_name(0))
  end
end

print('queries: ' .. table.concat(loaded, ', '))
print(('%d programas abertos, %d capturas de realce; dobra em "Para começar:" (%s)'):format(#files, captures, fold))
for _, line in ipairs(server) do
  print('servidor: ' .. line)
end
if #problems > 0 then
  print(('\n%d problemas:\n%s'):format(#problems, table.concat(problems, '\n')))
  vim.cmd('cquit 1')
end
print('Neovim: tipo de arquivo, ftplugin, parser, queries e realce em todos os programas, sem erro.')
-- Sem o qa!: o -l sai sozinho no fim do roteiro, com o código 0 e a última linha completa.
