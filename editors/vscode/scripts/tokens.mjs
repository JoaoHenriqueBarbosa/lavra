// Mostra os tokens e escopos que a gramática dá a um arquivo .lav, com o mesmo motor
// do VS Code (vscode-textmate + Oniguruma). Uso: node scripts/tokens.mjs arquivo.lav
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const vsctm = require("vscode-textmate");
const oniguruma = require("vscode-oniguruma");

const wasm = fs.readFileSync(path.join(path.dirname(require.resolve("vscode-oniguruma")), "onig.wasm"));
await oniguruma.loadWASM(wasm.buffer);
const registry = new vsctm.Registry({
  onigLib: Promise.resolve({ createOnigScanner: (p) => new oniguruma.OnigScanner(p), createOnigString: (s) => new oniguruma.OnigString(s) }),
  loadGrammar: async () => vsctm.parseRawGrammar(fs.readFileSync(new URL("../syntaxes/lavra.tmLanguage.json", import.meta.url), "utf8"), "lavra.tmLanguage.json"),
});
const grammar = await registry.loadGrammar("source.lavra");
let state = vsctm.INITIAL;
for (const line of fs.readFileSync(process.argv[2], "utf8").split("\n")) {
  const r = grammar.tokenizeLine(line, state);
  const parts = r.tokens
    .filter((t) => line.slice(t.startIndex, t.endIndex).trim())
    .map((t) => `${line.slice(t.startIndex, t.endIndex)}⟨${t.scopes.slice(1).map((s) => s.replace(/\.lavra$/, "")).join(" ")}⟩`);
  if (parts.length) console.log(parts.join(" "));
  state = r.ruleStack;
}
