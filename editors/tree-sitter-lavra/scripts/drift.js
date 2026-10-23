#!/usr/bin/env node
// Teste de deriva: a gramática acompanha a língua que o compilador aceita.
//
// - Todo programa certo do corpus (`samples/` e `tests/programs/ok/`) parseia sem nó ERROR
//   nem MISSING.
// - Cada oração que abre uma linha está na coluna do bloco em que a gramática a pôs, quatro
//   espaços por bloco: o corpus está na forma do formatador (`lavrac fmt --check` no CI), que
//   recua cada linha lógica pela profundidade que o compilador calcula. Assim, um bloco que a
//   gramática abrisse ou fechasse fora de lugar aparece aqui, mesmo sem erro de sintaxe.
// - Toda linha de comentário ("Nota:" ou "--" no começo) vira um nó `comment`.
// - Os programas de teste da gramática (`test/programs/`, Lavra válida conferida pelo
//   `cargo test` do compilador: tabs, CRLF, sem quebra final, maiúsculas) e as fixtures da
//   integração do VS Code (`editors/vscode/tests/integration/fixture/`, programas bem
//   formados; o erro de escopo de uma delas a gramática rasa não vê, de propósito) parseiam
//   sem erro e com os comentários; a forma deles não passa pelo formatador no CI, então a
//   conferência da coluna não vale ali.
// - Os programas errados (`tests/programs/err/`) não derrubam o parser.
//
// Uso: node scripts/drift.js (dentro de editors/tree-sitter-lavra, com o `tree-sitter` no
// PATH ou na variável TREE_SITTER).

'use strict';

const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const grammarDir = path.resolve(__dirname, '..');
const root = path.resolve(grammarDir, '..', '..');
const treeSitter = process.env.TREE_SITTER || 'tree-sitter';

// As construções que são uma oração ou abrem um bloco.
const STATEMENTS = new Set([
  'sentence',
  'import',
  'if_statement',
  'else_statement',
  'while_statement',
  'repeat_statement',
  'for_statement',
  'match_statement',
  'match_arm',
  'function_definition',
  'main_definition',
  'type_definition',
  'variant',
]);

function lavFiles(dir) {
  return fs
    .readdirSync(dir)
    .filter((f) => f.endsWith('.lav'))
    .sort()
    .map((f) => path.join(dir, f));
}

/** As árvores do `tree-sitter parse`, uma por arquivo, na ordem dos arquivos. */
function parse(files) {
  const run = spawnSync(treeSitter, ['parse', ...files], {
    cwd: grammarDir,
    encoding: 'utf8',
    maxBuffer: 256 * 1024 * 1024,
    timeout: 120000,
  });
  if (run.error) {
    throw run.error;
  }
  if (run.signal) {
    throw new Error(`o tree-sitter caiu com o sinal ${run.signal}`);
  }
  const trees = [];
  for (const line of run.stdout.split('\n')) {
    if (line.startsWith('(source_file')) {
      trees.push([]);
    }
    if (trees.length > 0 && /^\s*(\w+: )?\(/.test(line)) {
      trees[trees.length - 1].push(line);
    }
  }
  if (trees.length !== files.length) {
    throw new Error(`esperava ${files.length} árvores e o tree-sitter deu ${trees.length}:\n${run.stderr}`);
  }
  return trees;
}

const NODE = /^(\s*)(?:\w+: )?\((\w+) \[(\d+), (\d+)\] - \[(\d+), (\d+)\]/;

const counted = { statements: 0, comments: 0 };

/**
 * Os problemas de um programa certo: erros de sintaxe, comentário perdido e, com `columns`,
 * oração fora da coluna do bloco.
 */
function check(file, tree, columns) {
  const problems = [];
  const lines = fs.readFileSync(file, 'utf8').split('\n');
  const where = (row, col) => `${path.relative(root, file)}:${row + 1}:${col + 1}`;
  const comments = new Set();
  const stack = [];
  for (const line of tree) {
    if (/\((ERROR|MISSING)\b/.test(line)) {
      const m = line.match(/\[(\d+), (\d+)\]/);
      problems.push(`${where(Number(m[1]), Number(m[2]))}: ${line.trim()}`);
      continue;
    }
    const m = line.match(NODE);
    if (!m) {
      continue;
    }
    const level = m[1].length / 2;
    const [type, row, col] = [m[2], Number(m[3]), Number(m[4])];
    while (stack.length > level) {
      stack.pop();
    }
    stack.push(type);
    if (type === 'comment') {
      comments.add(`${row}:${col}`);
    }
    // Só a oração que abre a linha: a que vem depois dos dois-pontos, na mesma linha, não.
    const indent = lines[row].length - lines[row].trimStart().length;
    if (columns && STATEMENTS.has(type) && col === indent) {
      counted.statements++;
      const depth = stack.filter((t) => t === 'block').length;
      if (col !== depth * 4) {
        problems.push(`${where(row, col)}: ${type} na profundidade ${depth} da gramática, recuado ${col} colunas`);
      }
    }
  }
  lines.forEach((text, row) => {
    const content = text.trimStart();
    if (content.toLowerCase().startsWith('nota:') || content.startsWith('--')) {
      const col = text.length - content.length;
      counted.comments++;
      if (!comments.has(`${row}:${col}`)) {
        problems.push(`${where(row, col)}: a linha de comentário não virou um nó comment`);
      }
    }
  });
  return problems;
}

const ok = [...lavFiles(path.join(root, 'samples')), ...lavFiles(path.join(root, 'tests', 'programs', 'ok'))];
const own = lavFiles(path.join(grammarDir, 'test', 'programs'));
const fixtures = lavFiles(path.join(root, 'editors', 'vscode', 'tests', 'integration', 'fixture'));
const err = lavFiles(path.join(root, 'tests', 'programs', 'err'));

const problems = [];
parse(ok).forEach((tree, i) => problems.push(...check(ok[i], tree, true)));
parse(own).forEach((tree, i) => problems.push(...check(own[i], tree, false)));
parse(fixtures).forEach((tree, i) => problems.push(...check(fixtures[i], tree, false)));
// Os errados só precisam terminar com uma árvore cada um, sem derrubar o parser.
parse(err);

if (problems.length > 0) {
  console.error(problems.join('\n'));
  console.error(`\n${problems.length} problemas em ${ok.length + own.length + fixtures.length} programas.`);
  process.exit(1);
}
console.log(
  `${ok.length} programas do corpus, ${own.length} da gramática e ${fixtures.length} fixtures do VS Code sem erro, ` +
    `com ${counted.statements} orações no recuo do compilador e ${counted.comments} linhas de comentário; ` +
    `${err.length} programas errados sem derrubar o parser.`,
);
