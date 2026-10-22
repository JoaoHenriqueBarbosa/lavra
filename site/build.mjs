// Gera o site estático da Lavra em site/dist, a partir do que o repositório já tem: a
// referência (notes/lavra.md), o catálogo de erros (src/codes.rs e src/explain), o léxico
// (editors/lexicon.json), os poemas (samples/) e o playground com o interpretador em wasm
// (target/playground/web, que o crate playground/ gera). O código da Lavra sai realçado
// pela mesma gramática da extensão do VS Code.
import fs from "node:fs";
import path from "node:path";
import * as esbuild from "esbuild";
import { Marked } from "marked";
import { createHighlighter } from "shiki";

const here = import.meta.dirname;
const root = path.resolve(here, "..");
const out = path.join(here, "dist");
const read = (p) => fs.readFileSync(path.join(root, p), "utf8");
const exists = (p) => fs.existsSync(path.join(root, p));
const wasm = path.resolve(process.env.LAVRA_WASM_PKG ?? path.join(root, "target/playground/web"));
const repo = "https://github.com/JoaoHenriqueBarbosa/lavra";

if (!fs.existsSync(path.join(wasm, "lavra_playground_bg.wasm"))) {
  console.error(`falta o interpretador em wasm em ${wasm}: compile o crate playground/ antes (ver notes/tooling.md)`);
  process.exit(1);
}

const grammar = JSON.parse(read("editors/vscode/syntaxes/lavra.tmLanguage.json"));
const lexicon = JSON.parse(read("editors/lexicon.json"));
const highlighter = await createHighlighter({ themes: ["light-plus", "dark-plus"], langs: [{ ...grammar, name: "lavra" }] });
const highlight = (code) =>
  highlighter.codeToHtml(code.replace(/\n$/, ""), { lang: "lavra", themes: { light: "light-plus", dark: "dark-plus" }, defaultColor: false });

const escape = (s) => s.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;");

/** Markdown com os blocos de código realçados como Lavra (os da referência e das explicações). */
function markdown() {
  const m = new Marked();
  m.use({
    renderer: {
      code({ text, lang }) {
        if (!lang || lang === "lavra") return highlight(text);
        return `<pre><code>${escape(text)}</code></pre>`;
      },
      heading({ tokens, depth }) {
        const text = this.parser.parseInline(tokens);
        const id = slug(text.replace(/<[^>]+>/g, ""));
        return `<h${depth} id="${id}">${text}</h${depth}>\n`;
      },
    },
  });
  return m;
}

function slug(text) {
  return text
    .toLowerCase()
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
}

const NAV = [
  ["playground.html", "Playground"],
  ["poemas/index.html", "Poemas"],
  ["referencia.html", "Referência"],
  ["erros.html", "Erros"],
  ["lexico.html", "Léxico"],
];

function page({ file, title, description, body, wide = false, scripts = "" }) {
  const depth = file.split("/").length - 1;
  const up = "../".repeat(depth);
  const nav = NAV.map(([href, label]) => `<a href="${up}${href}"${href === file ? ' aria-current="page"' : ""}>${label}</a>`).join("");
  const html = `<!doctype html>
<html lang="pt-BR">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escape(title)}</title>
<meta name="description" content="${escape(description)}">
<link rel="stylesheet" href="${up}style.css">
</head>
<body>
<header class="top"><a class="brand" href="${up}index.html">Lavra</a><nav>${nav}<a href="${repo}">GitHub</a></nav></header>
<main${wide ? ' class="wide"' : ""}>
${body}
</main>
<footer>Lavra, a linguagem em que cada instrução é uma oração. Licença MIT. <a href="${repo}">Código</a> · <a href="${repo}/releases/latest">Baixar</a></footer>
${scripts}
</body>
</html>
`;
  const target = path.join(out, file);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.writeFileSync(target, html);
}

fs.rmSync(out, { recursive: true, force: true });
fs.mkdirSync(out, { recursive: true });
fs.copyFileSync(path.join(here, "src/style.css"), path.join(out, "style.css"));

// Os poemas: os samples com a saída esperada (e a entrada, quando o programa lê).
const samples = fs
  .readdirSync(path.join(root, "samples"))
  .filter((f) => f.endsWith(".lav"))
  .sort()
  .map((f) => {
    const name = f.slice(0, -4);
    const code = read(`samples/${f}`);
    const first = code.split("\n")[0].replace(/^Nota:\s*/i, "").replace(/\.$/, "");
    const title = first.charAt(0).toUpperCase() + first.slice(1);
    const input = exists(`samples/${name}.stdin`) ? read(`samples/${name}.stdin`) : "";
    const output = exists(`samples/${name}.stdout`) ? read(`samples/${name}.stdout`) : "";
    return { name, title, code, input, output };
  });
// O primeiro do playground é o mais curto que não lê nada: abre e roda sem pedir entrada.
const opening = samples.find((s) => s.name === "caixa-de-poupanca") ?? samples[0];
const playgroundSamples = [opening, ...samples.filter((s) => s !== opening)].map(({ name, title, code, input }) => ({ name, title, code, input }));

// A apresentação.
const example = `Para render o saldo, sendo o saldo um real confiado:
    O saldo cresce o saldo vezes 0,05.

Para começar:
    O saldo começa em 1000,0.
    Repito 2 vezes: rendo o saldo.
    Escrevo "rendido: " seguido do saldo.
`;
page({
  file: "index.html",
  title: "Lavra: a linguagem em que cada instrução é uma oração",
  description: "Uma linguagem de programação compilada em que cada instrução é uma oração em português, e a gramática é a verificação.",
  body: `<section class="hero">
<h1>Lavra</h1>
<p class="tagline">A linguagem compilada em que cada instrução é uma oração em português, e a gramática é a verificação.</p>
${highlight(example)}
<p class="output-label">Saída</p>
<pre class="output">rendido: 1102,5</pre>
<div class="actions">
<a class="button primary" href="playground.html">Experimente no navegador</a>
<a class="button" href="${repo}/releases/latest">Baixar o lavrac</a>
<a class="button" href="referencia.html">Ler a referência</a>
</div>
</section>
<h2>A gramática é a verificação</h2>
<div class="grid">
<div class="card"><h3>O artigo marca o nome</h3><p>"o saldo", "as notas": o nome tem gênero e número, e cada uso concorda com a declaração.</p></div>
<div class="card"><h3>O verbo diz se muda</h3><p>"A taxa é 0,05." não muda; "O saldo começa em 0." promete mudar, e o compilador cobra.</p></div>
<div class="card"><h3>A regência é a assinatura</h3><p>"Transfiro 300 da conta para a poupança.": a preposição e a ordem são as do cabeçalho da função.</p></div>
<div class="card"><h3>A concordância é conferida</h3><p>Artigo, verbo, particípio, adjetivo e comparação: "Se as notas forem maiores que 5".</p></div>
</div>
<h2>Um Rust com gramática e morfologia</h2>
<p>Posse e empréstimo ("emprestado", "confiado"), "talvez" e falha ("salvo uma falha"), registros e enumerações com "Conforme", bibliotecas ("Uso a biblioteca de textos."). O interpretador e o compilador para executável nativo dão a mesma saída, byte a byte. Os erros dizem o que a gramática espera, cada um com o código da <a href="erros.html">explicação</a>, e muitos com a correção que o compilador aplica sozinho.</p>
<p>E os programas podem ser <a href="poemas/index.html">poemas</a>.</p>
<h2>Instalar</h2>
<p>Na página da <a href="${repo}/releases/latest">última versão</a> estão o <code>lavrac</code> pronto para Linux (x86_64, o PC comum; aarch64, os computadores ARM), que roda sem instalar mais nada, e a extensão do VS Code, que já traz o <code>lavrac</code> dentro: os erros ao digitar, o completar guiado pela gramática, o rename que acerta o artigo, o depurador. Com o Rust instalado: <code>cargo install --git ${repo} lavrac</code>.</p>`,
});

// A referência da língua.
{
  const md = read("notes/lavra.md");
  const tokens = new Marked().lexer(md);
  const sections = tokens.filter((t) => t.type === "heading" && t.depth === 2).map((t) => `<li><a href="#${slug(t.text)}">${escape(t.text)}</a></li>`);
  const body = markdown().parse(md).replace(/<h1[^>]*>.*?<\/h1>/, (h1) => `${h1}\n<nav><ul class="toc">${sections.join("")}</ul></nav>`);
  page({ file: "referencia.html", title: "Referência da Lavra", description: "A língua como o compilador a implementa.", body });
}

// O catálogo de erros, por categoria, com a explicação de cada código.
{
  const CATEGORY = {
    Lexical: "Erros léxicos",
    Morphology: "Erros de morfologia",
    Agreement: "Erros de concordância",
    Regency: "Erros de regência",
    Syntax: "Erros de sintaxe",
    Scope: "Erros de escopo",
    Type: "Erros de tipo",
    Mutability: "Erros de mutabilidade",
    Ownership: "Erros de posse",
    Borrow: "Erros de empréstimo",
    Runtime: "Erros de execução",
    Warning: "Avisos",
  };
  const codes = [...read("src/codes.rs").matchAll(/^\s*(L\d{4}) = (\w+), "((?:[^"\\]|\\.)*)";/gm)].map(([, id, category, title]) => ({
    id,
    category,
    title: title.replaceAll('\\"', '"'),
  }));
  const md = markdown();
  const groups = Object.entries(CATEGORY).map(([category, label]) => {
    const list = codes.filter((c) => c.category === category);
    if (!list.length) return "";
    const entries = list.map((c) => {
      const file = `src/explain/${c.id}.md`;
      const text = exists(file) ? read(file).replace(/^# .*\n/, "") : "";
      return `<section class="entry" id="${c.id}"><h3><code>${c.id}</code> ${escape(c.title)}</h3>\n${md.parse(text.replaceAll("\n## ", "\n#### "))}</section>`;
    });
    return `<h2 id="${slug(label)}">${label} <span class="badge">${list.length}</span></h2>\n${entries.join("\n")}`;
  });
  const toc = Object.entries(CATEGORY)
    .filter(([category]) => codes.some((c) => c.category === category))
    .map(([, label]) => `<li><a href="#${slug(label)}">${label}</a></li>`)
    .join("");
  page({
    file: "erros.html",
    title: "Os erros da Lavra",
    description: "Cada diagnóstico do compilador, com a explicação, um exemplo errado e um certo.",
    body: `<h1>Os erros da Lavra</h1>
<p>Cada diagnóstico do compilador tem um código estável, e cada código tem a explicação, com um exemplo errado e um certo. No terminal, <code>lavrac explain L0334</code> mostra o mesmo texto; no editor, o código do erro leva para cá. São ${codes.length} códigos.</p>
<nav><ul class="toc">${toc}</ul></nav>
${groups.join("\n")}`,
  });
}

// O léxico: as palavras da língua, pelo papel, com a documentação de cada uma.
{
  const roles = Object.fromEntries(lexicon.roles.map((r) => [r.id, r.name]));
  const row = (word, doc) => `<tr><td><code>${escape(word)}</code></td><td>${escape(doc ?? "")}</td></tr>`;
  const table = (rows) => `<table><thead><tr><th>Palavra</th><th>O que faz</th></tr></thead><tbody>${rows.join("")}</tbody></table>`;
  const byRole = {};
  for (const k of lexicon.keywords) (byRole[k.role] ??= []).push(k);
  const keywordSections = Object.entries(byRole)
    .filter(([role]) => role !== "rejected")
    .map(([role, list]) => `<h3>${escape(roles[role] ?? role)}</h3>${table(list.map((k) => row(k.form, k.doc)))}`)
    .join("");
  const body = `<h1>O léxico da Lavra</h1>
<p>As palavras da língua, como o compilador as conhece: é daqui que saem as palavras reservadas, o realce dos editores e a documentação do hover. Os verbos do programa e os nomes não estão aqui: são do programa.</p>
<h2>Verbos da língua</h2>
<p>Na 1ª pessoa: o programa narra quando age no mundo.</p>
${table(lexicon.coreVerbs.map((v) => row(`${v.firstPerson} (${v.infinitive})`, v.doc)))}
<h2>Verbos do sujeito</h2>
<p>O dado é o sujeito do que acontece com ele, e o verbo concorda.</p>
${table(lexicon.subjectVerbs.map((v) => row(`${v.singular} / ${v.plural}`, v.doc)))}
<h2>Tipos</h2>
${table(lexicon.types.map((t) => row(`${t.singular} / ${t.plural}`, t.doc)))}
<h2>Operações por extenso</h2>
${table(lexicon.nominals.map((n) => row(`${n.gender === "feminine" ? "a" : "o"} ${n.word}`, n.doc)))}
<h2>Posse</h2>
${table(lexicon.participles.map((p) => row(p.forms.join(", "), p.doc)))}
<h2>Palavras da língua, pelo papel</h2>
${keywordSections}
<h2>Artigos, contrações e preposições</h2>
<p>Artigos: ${lexicon.articles.map((a) => `<code>${a.form}</code>`).join(", ")}.</p>
<p>Contrações (obrigatórias: "de o" é erro, "do" é certo): ${lexicon.contractions.map((c) => `<code>${c.form}</code> (${c.preposition} + ${c.article})`).join(", ")}.</p>
<p>Preposições da regência: ${lexicon.prepositions.map((p) => `<code>${p}</code>`).join(", ")}.</p>
<p>Ordinais: ${lexicon.ordinals.map((o) => `<code>${o.stem}o</code>`).join(", ")}.</p>`;
  page({ file: "lexico.html", title: "O léxico da Lavra", description: "As palavras da língua, pelo papel, com a documentação de cada uma.", body });
}

// Os poemas: a lista e uma página por poema, com o código, a entrada e a saída.
page({
  file: "poemas/index.html",
  title: "Os poemas da Lavra",
  description: "Programas que são poemas, e poemas que compilam.",
  body: `<h1>Os poemas</h1>
<p>Programas que são poemas, e poemas que compilam: cada um roda no <a href="../playground.html">playground</a> e dá a mesma saída no interpretador e no executável nativo.</p>
<ul class="poems">${samples
    .map((s) => `<li><a href="${s.name}.html">${escape(s.title)}</a> <span class="what">${s.code.split("\n").length} linhas</span></li>`)
    .join("")}</ul>`,
});
for (const s of samples) {
  const inputBlock = s.input ? `<p class="output-label">Entrada</p><pre class="output">${escape(s.input)}</pre>` : "";
  const outputBlock = s.output ? `<p class="output-label">Saída</p><pre class="output">${escape(s.output)}</pre>` : "";
  page({
    file: `poemas/${s.name}.html`,
    title: `${s.title} · Lavra`,
    description: `${s.title}: um programa em Lavra.`,
    body: `<h1>${escape(s.title)}</h1>
<div class="actions"><a class="button primary" href="../playground.html?poema=${s.name}&amp;rodar">Rodar no playground</a><a class="button" href="${repo}/blob/main/samples/${s.name}.lav">Ver no GitHub</a></div>
${highlight(s.code)}
${inputBlock}
${outputBlock}`,
  });
}

// O playground: o editor, o interpretador em wasm num worker e os exemplos.
await esbuild.build({
  entryPoints: [path.join(here, "src/playground.js")],
  bundle: true,
  format: "esm",
  minify: true,
  target: "es2022",
  outfile: path.join(out, "playground/playground.js"),
  loader: { ".json": "json" },
  logLevel: "warning",
});
fs.copyFileSync(path.join(here, "src/worker.js"), path.join(out, "playground/worker.js"));
fs.mkdirSync(path.join(out, "playground/pkg"), { recursive: true });
for (const f of ["lavra_playground.js", "lavra_playground_bg.wasm"]) {
  fs.copyFileSync(path.join(wasm, f), path.join(out, "playground/pkg", f));
}
page({
  file: "playground.html",
  title: "Playground da Lavra",
  description: "Escreva e rode programas em Lavra no navegador, com os erros do compilador ao digitar.",
  wide: true,
  body: `<h1>Playground</h1>
<p>O compilador da Lavra roda aqui, no navegador: os erros aparecem enquanto você escreve, e o programa roda no interpretador. Ctrl+Enter roda.</p>
<div class="toolbar">
<label class="status" for="exemplo">Exemplo</label>
<select id="exemplo"></select>
<button class="button primary" id="rodar" type="button">Rodar</button>
<button class="button" id="parar" type="button" disabled>Parar</button>
<button class="button" id="compartilhar" type="button">Compartilhar</button>
<span class="status" id="status" role="status"></span>
</div>
<div class="playground">
<div class="editor" id="editor"></div>
<div class="panes">
<section class="pane"><h2><label for="entrada">Entrada</label></h2><textarea id="entrada" spellcheck="false" placeholder="O que o programa lê com &quot;Leio&quot;, uma linha por leitura."></textarea></section>
<section class="pane"><h2>Saída</h2><pre id="saida" aria-live="polite"></pre></section>
<section class="pane"><h2>Diagnósticos</h2><pre id="problemas" class="problems clean"></pre></section>
</div>
</div>
<script id="samples" type="application/json">${JSON.stringify(playgroundSamples).replaceAll("<", "\\u003c")}</script>`,
  scripts: `<script type="module" src="playground/playground.js"></script>`,
});

console.log(`site em ${path.relative(root, out)}: ${samples.length} poemas`);
