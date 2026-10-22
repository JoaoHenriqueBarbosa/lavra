// O interpretador do playground (wasm) contra o corpus: cada sample e cada programa certo,
// com a entrada dele, dá a saída esperada, como no lavrac. Também os diagnósticos e uma
// recursão perto do limite de profundidade da língua.
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";

const root = path.resolve(import.meta.dirname, "..");
const pkg = process.argv[2] ?? path.join(root, "target/playground/node/lavra_playground.js");
const { check, run } = createRequire(import.meta.url)(pkg);

let failures = 0;
const fail = (msg) => {
  console.error(`falhou: ${msg}`);
  failures++;
};

function execute(source, input) {
  let stdout = "";
  const result = JSON.parse(run(source, input, (text) => (stdout += text)));
  return { ...result, stdout };
}

let checked = 0;
for (const dir of ["samples", "tests/programs/ok"]) {
  for (const f of fs.readdirSync(path.join(root, dir)).filter((f) => f.endsWith(".lav")).sort()) {
    const base = path.join(root, dir, f.slice(0, -4));
    if (!fs.existsSync(`${base}.stdout`)) continue;
    const input = fs.existsSync(`${base}.stdin`) ? fs.readFileSync(`${base}.stdin`, "utf8") : "";
    const r = execute(fs.readFileSync(`${base}.lav`, "utf8"), input);
    if (r.stdout !== fs.readFileSync(`${base}.stdout`, "utf8")) fail(`${dir}/${f}: a saída difere`);
    checked++;
  }
}

// Diagnósticos com o trecho em unidades UTF-16 (o "ç" de "começar" conta um).
const broken = "Para começar:\n    Escrevo o sald.\n";
const diags = JSON.parse(check(broken));
if (diags.length !== 1 || diags[0].code !== "L0334") fail(`check: ${JSON.stringify(diags)}`);
else if (broken.slice(diags[0].from, diags[0].to) !== "o sald") fail(`check: o trecho é "${broken.slice(diags[0].from, diags[0].to)}"`);
if (!diags[0]?.rendered.includes("[L0334]")) fail("check: o texto do diagnóstico");
const errors = execute(broken, "");
if (errors.status !== "errors" || !errors.stderr.includes("L0334")) fail(`run com erro: ${JSON.stringify(errors)}`);

// Recursão funda: no navegador cabem 1 000 chamadas aninhadas, e passar disso é a falha da
// própria língua, nunca o estouro do motor JavaScript.
const deep = (n) =>
  `Para descer n, sendo n um inteiro, devolvendo um inteiro:\n    Se n for igual a 0: devolvo 0.\n    Devolvo o resultado de descer n menos 1.\n\nPara começar:\n    Escrevo o resultado de descer ${n}.\n`;
const ok = execute(deep(990), "");
if (ok.status !== "ok" || ok.stdout !== "0\n") fail(`recursão de 990: ${JSON.stringify(ok).slice(0, 200)}`);
const over = execute(deep(20000), "");
if (over.status !== "failed" || !over.stderr.includes("Estouro de pilha: chamadas demais aninhadas (mais de 1000)"))
  fail(`recursão de 20 000: ${JSON.stringify(over).slice(0, 200)}`);

console.log(`${checked} programas com a saída esperada; ${failures} falhas`);
process.exit(failures ? 1 : 0);
