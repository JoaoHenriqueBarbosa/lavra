#!/usr/bin/env node
// Toda query compila contra a gramática: as daqui (`queries/`) e as de cada editor (as cópias
// do sync-queries.js e as escritas à mão, como o recuo do Helix e o esboço do Zed). Uma
// query com nó, campo ou captura que a gramática não tem só falharia dentro do editor; aqui
// falha antes, rodando sobre os programas de teste da gramática.
//
// Uso: node scripts/check-queries.js (dentro de editors/tree-sitter-lavra, com o
// `tree-sitter` no PATH ou na variável TREE_SITTER).

'use strict';

const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const grammarDir = path.resolve(__dirname, '..');
const editors = path.resolve(grammarDir, '..');
const treeSitter = process.env.TREE_SITTER || 'tree-sitter';

const DIRS = [
  path.join(grammarDir, 'queries'),
  path.join(editors, 'helix', 'runtime', 'queries', 'lavra'),
  path.join(editors, 'neovim', 'queries', 'lavra'),
  path.join(editors, 'zed', 'languages', 'lavra'),
];

const programs = fs
  .readdirSync(path.join(grammarDir, 'test', 'programs'))
  .filter((f) => f.endsWith('.lav'))
  .sort()
  .map((f) => path.join('test', 'programs', f));

let failures = 0;
let count = 0;
for (const dir of DIRS) {
  for (const file of fs.readdirSync(dir).filter((f) => f.endsWith('.scm')).sort()) {
    const query = path.join(dir, file);
    const run = spawnSync(treeSitter, ['query', '--quiet', query, ...programs], { cwd: grammarDir, encoding: 'utf8' });
    count++;
    if (run.error || run.status !== 0) {
      failures++;
      console.error(`${path.relative(editors, query)}:\n${run.error ? run.error.message : run.stderr.trim()}\n`);
    }
  }
}
if (failures > 0) {
  console.error(`${failures} de ${count} queries não compilam contra a gramática.`);
  process.exit(1);
}
console.log(`${count} queries compilam contra a gramática, sobre ${programs.length} programas.`);
