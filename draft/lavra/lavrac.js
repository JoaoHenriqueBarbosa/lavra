#!/usr/bin/env node
// lavrac — linha de comando da Lavra.
//   node lavrac.js programa.lav            interpreta (lê o teclado normalmente)
//   node lavrac.js -s programa.lav         escreve programa.s (assembly x86-64)
//   node lavrac.js -c programa.lav         gera o executável nativo ./programa (precisa de gcc em Linux x86-64)
const fs = require('fs'), path = require('path'), { execFileSync } = require('child_process');
const L = require('./lavra.js'); require('./x86.js');
const args = process.argv.slice(2);
const modo = args[0] === '-s' ? 's' : args[0] === '-c' ? 'c' : 'i';
const arq = modo === 'i' ? args[0] : args[1];
if (!arq) { console.error('uso: node lavrac.js [-s|-c] programa.lav'); process.exit(2); }
const fonte = fs.readFileSync(arq, 'utf8');
const c = L.compilar(fonte);
if (!c.ok) {
  const e = c.erro;
  console.error(`${arq}:${e.linha || '?'}: ${L.ROTULO_ERRO[e.tipo]}: ${e.message}${e.dica ? '\n  dica: ' + e.dica : ''}`);
  process.exit(1);
}
if (modo === 'i') {
  const linhas = fs.readFileSync(0, 'utf8').split('\n');
  const m = new L.Maquina(c.prog, { escreve: s => process.stdout.write(s), aviso: s => console.error('[aviso] ' + s) });
  const g = m.executar();
  try {
    let r = g.next();
    while (!r.done) r = g.next(r.value.k === 'ler' ? (linhas.length ? linhas.shift() : null) : undefined);
  } catch (err) {
    const e = L.traduzirErro(err, m); if (!e) throw err;
    console.error(`\n[${L.ROTULO_ERRO[e.tipo]}] linha ${e.linha}: ${e.message}`); process.exit(1);
  }
  process.exit(0);
}
const g = L.gerarX86(c.prog, fonte);
if (!g.ok) { console.error('gerador x86: ' + g.erro); process.exit(1); }
const base = arq.replace(/\.lav$/, '');
fs.writeFileSync(base + '.s', g.asm);
if (modo === 's') { console.log(`escrito ${base}.s`); process.exit(0); }
execFileSync('gcc', [base + '.s', path.join(__dirname, 'runtime.c'), '-o', base, '-lm'], { stdio: 'inherit' });
console.log(`executável: ${base}`);
