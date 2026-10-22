// Renderiza samples com a gramática da Lavra nos temas Dark+ e Light+ do VS Code
// (shiki usa o mesmo motor TextMate). Uso: node scripts/preview.mjs saída.html arq.lav...
import fs from "node:fs";
import path from "node:path";
import { createHighlighter } from "shiki";

const [out, ...files] = process.argv.slice(2);
const grammar = JSON.parse(fs.readFileSync(new URL("../syntaxes/lavra.tmLanguage.json", import.meta.url), "utf8"));
const hl = await createHighlighter({ themes: ["dark-plus", "light-plus"], langs: [{ ...grammar, name: "lavra" }] });
const blocks = files
  .map((f) => {
    const code = fs.readFileSync(f, "utf8");
    const pair = ["dark-plus", "light-plus"].map((theme) => hl.codeToHtml(code, { lang: "lavra", theme })).join("");
    return `<h2>${path.basename(f)}</h2><div class="pair">${pair}</div>`;
  })
  .join("");
fs.writeFileSync(
  out,
  `<!doctype html><meta charset="utf-8"><title>Lavra: realce</title>
<style>body{font-family:sans-serif;margin:16px;background:#888}h2{font-size:14px;margin:12px 0 4px}
.pair{display:grid;grid-template-columns:1fr 1fr;gap:8px}pre{margin:0;padding:10px;font-size:13px;line-height:1.45;overflow:auto;font-family:"Cascadia Code",Consolas,monospace}</style>${blocks}`,
);
console.log(`escrevi ${out}`);
