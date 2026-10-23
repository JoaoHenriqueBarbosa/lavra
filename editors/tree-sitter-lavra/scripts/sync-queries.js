#!/usr/bin/env node
// As cópias das queries de cada editor, a partir das queries daqui (`queries/`, com os nomes
// de captura do Neovim). Os três editores resolvem do mesmo jeito dois padrões sobre o mesmo
// trecho (vence o que vem depois), então só os nomes de captura mudam.
//
// Uso: node scripts/sync-queries.js          escreve as cópias
//      node scripts/sync-queries.js --check  só confere que estão em dia (o CI usa)

'use strict';

const fs = require('node:fs');
const path = require('node:path');

const grammarDir = path.resolve(__dirname, '..');
const editors = path.resolve(grammarDir, '..');

// Os nomes do Helix (https://docs.helix-editor.com/themes.html#scopes).
const HELIX = {
  variable: 'variable',
  'variable.builtin': 'variable.builtin',
  'variable.parameter': 'variable.parameter',
  'variable.member': 'variable.other.member',
  keyword: 'keyword',
  'keyword.operator': 'keyword.operator',
  'keyword.return': 'keyword.control.return',
  'keyword.repeat': 'keyword.control.repeat',
  'keyword.conditional': 'keyword.control.conditional',
  'keyword.function': 'keyword.function',
  'keyword.import': 'keyword.control.import',
  'keyword.type': 'keyword.storage.type',
  'keyword.modifier': 'keyword.storage.modifier',
  function: 'function',
  'function.builtin': 'function.builtin',
  'function.call': 'function',
  type: 'type',
  'type.builtin': 'type.builtin',
  'type.definition': 'type',
  constant: 'type.enum.variant',
  'constant.builtin': 'constant.builtin',
  module: 'namespace',
  number: 'constant.numeric',
  string: 'string',
  'string.escape': 'constant.character.escape',
  character: 'constant.character',
  comment: 'comment',
  'punctuation.delimiter': 'punctuation.delimiter',
  'punctuation.bracket': 'punctuation.bracket',
  'local.scope': 'local.scope',
  'local.reference': 'local.reference',
  'local.definition.var': 'local.definition.variable',
  'local.definition.parameter': 'local.definition.variable.parameter',
  'local.definition.function': 'local.definition.function',
  'local.definition.type': 'local.definition.type',
  'function.inside': 'function.inside',
  'function.around': 'function.around',
  'class.inside': 'class.inside',
  'class.around': 'class.around',
  'parameter.inside': 'parameter.inside',
  'comment.inside': 'comment.inside',
  'comment.around': 'comment.around',
};

// O Zed acha o estilo pelo prefixo do nome ("keyword.conditional" usa "keyword"); só muda o
// que não tem prefixo no tema dele (https://zed.dev/docs/extensions/languages).
const ZED = {
  'variable.builtin': 'variable.special',
  'variable.member': 'property',
  constant: 'variant',
  module: 'type',
  character: 'string.special',
};

// O Neovim usa os nomes daqui; o comentário ganha a checagem ortográfica, e os objetos de
// texto do nvim-treesitter-textobjects se chamam "inner" e "outer".
const NEOVIM = {
  comment: ['comment', 'spell'],
  'function.inside': 'function.inner',
  'function.around': 'function.outer',
  'class.inside': 'class.inner',
  'class.around': 'class.outer',
  'parameter.inside': 'parameter.inner',
  'comment.inside': 'comment.inner',
  'comment.around': 'comment.outer',
};

/** As capturas da query trocadas pelos nomes do editor. */
function mapCaptures(text, table, strict, file) {
  return text.replace(/@([\w.]+)/g, (whole, name) => {
    let mapped = table[name];
    if (mapped === undefined) {
      if (strict) {
        throw new Error(`${file}: a captura @${name} não tem nome no Helix; acrescente em scripts/sync-queries.js`);
      }
      mapped = name;
    }
    return (Array.isArray(mapped) ? mapped : [mapped]).map((m) => `@${m}`).join(' ');
  });
}

/** O cabeçalho que diz de onde a cópia veio. */
function header(source) {
  return `; Gerado de editors/tree-sitter-lavra/queries/${source} por scripts/sync-queries.js: não edite aqui.\n\n`;
}

/** Tira o comentário de abertura da query original (ele fala da original). */
function body(text) {
  const lines = text.split('\n');
  let i = 0;
  while (i < lines.length && lines[i].startsWith(';')) i++;
  while (i < lines.length && lines[i].trim() === '') i++;
  return lines.slice(i).join('\n');
}

const COPIES = [
  { editor: 'helix', dir: 'helix/runtime/queries/lavra', files: ['highlights.scm', 'locals.scm', 'textobjects.scm'], table: HELIX, strict: true },
  { editor: 'neovim', dir: 'neovim/queries/lavra', files: ['highlights.scm', 'locals.scm', 'folds.scm', 'indents.scm', 'textobjects.scm'], table: NEOVIM },
  { editor: 'zed', dir: 'zed/languages/lavra', files: ['highlights.scm', 'textobjects.scm'], table: ZED },
];

const check = process.argv.includes('--check');
const stale = [];
for (const { dir, files, table, strict } of COPIES) {
  for (const file of files) {
    const source = fs.readFileSync(path.join(grammarDir, 'queries', file), 'utf8');
    const target = path.join(editors, dir, file);
    const wanted = header(file) + mapCaptures(body(source), table, strict, file);
    const current = fs.existsSync(target) ? fs.readFileSync(target, 'utf8') : null;
    if (current === wanted) continue;
    if (check) {
      stale.push(path.relative(path.resolve(editors, '..'), target));
    } else {
      fs.mkdirSync(path.dirname(target), { recursive: true });
      fs.writeFileSync(target, wanted);
      console.log(`escrevi ${path.relative(path.resolve(editors, '..'), target)}`);
    }
  }
}
if (stale.length > 0) {
  console.error(`As cópias das queries estão desatualizadas (rode node scripts/sync-queries.js):\n${stale.join('\n')}`);
  process.exit(1);
}
