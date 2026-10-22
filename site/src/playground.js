// O playground: o editor com a língua e os diagnósticos do compilador ao digitar, e o
// programa rodando no interpretador em wasm, num worker (a saída vai saindo; parar encerra
// o worker). Os exemplos são os poemas do repositório.
import { indentWithTab } from "@codemirror/commands";
import { indentUnit, syntaxHighlighting } from "@codemirror/language";
import { linter, lintGutter } from "@codemirror/lint";
import { EditorView, keymap } from "@codemirror/view";
import { basicSetup } from "codemirror";
import lexicon from "../../editors/lexicon.json";
import { lavra, lavraHighlight } from "./lavra-language.js";

const $ = (sel) => document.querySelector(sel);
const samples = JSON.parse($("#samples").textContent);
const select = $("#exemplo");
const runButton = $("#rodar");
const stopButton = $("#parar");
const shareButton = $("#compartilhar");
const input = $("#entrada");
const output = $("#saida");
const problems = $("#problemas");
const status = $("#status");
const STORE = "lavra-playground";
const MAX_OUTPUT = 200_000;

const workerUrl = new URL("./worker.js", import.meta.url);

// Conferir: um worker só, que responde pelo número do pedido.
const checker = new Worker(workerUrl, { type: "module" });
const waiting = new Map();
let asked = 0;
checker.onmessage = (event) => {
  const done = waiting.get(event.data.id);
  if (done) {
    waiting.delete(event.data.id);
    done(event.data.diagnostics);
  }
};
const check = (source) =>
  new Promise((resolve) => {
    asked += 1;
    waiting.set(asked, resolve);
    checker.postMessage({ kind: "check", id: asked, source });
  });

function showProblems(diagnostics) {
  const text = diagnostics.map((d) => d.rendered).join("");
  problems.textContent = text || "Nenhum problema: o compilador aceita o programa.";
  problems.classList.toggle("clean", !text);
}

const lavraLint = linter(
  async (view) => {
    const diagnostics = await check(view.state.doc.toString());
    showProblems(diagnostics);
    const length = view.state.doc.length;
    return diagnostics.map((d) => ({
      from: Math.min(d.from, length),
      to: Math.min(Math.max(d.to, d.from), length),
      severity: d.severity,
      message: d.code ? `${d.message} [${d.code}]` : d.message,
    }));
  },
  { delay: 300 },
);

// Guardar o texto no navegador, para voltar a ele (sem garantia: pode faltar o armazenamento).
let saveTimer;
const keep = EditorView.updateListener.of((update) => {
  if (!update.docChanged) return;
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    try {
      localStorage.setItem(STORE, JSON.stringify({ code: update.state.doc.toString(), input: input.value }));
    } catch {}
  }, 500);
});

const view = new EditorView({
  parent: $("#editor"),
  extensions: [
    basicSetup,
    keymap.of([indentWithTab]),
    indentUnit.of("    "),
    lavra(lexicon),
    syntaxHighlighting(lavraHighlight),
    lavraLint,
    lintGutter(),
    keep,
    EditorView.contentAttributes.of({ spellcheck: "false", "aria-label": "Programa em Lavra" }),
  ],
});

function setCode(code, stdin = "") {
  view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: code } });
  input.value = stdin;
}

// Rodar: um worker por execução, para o botão de parar poder encerrá-lo.
let runner = null;
let buffer = "";
let flushing = false;

function flush() {
  flushing = false;
  if (!buffer) return;
  let text = output.textContent + buffer;
  buffer = "";
  if (text.length > MAX_OUTPUT) text = `(o começo da saída foi cortado)\n${text.slice(-MAX_OUTPUT)}`;
  output.textContent = text;
  output.scrollTop = output.scrollHeight;
}

function append(text) {
  buffer += text;
  if (!flushing) {
    flushing = true;
    requestAnimationFrame(flush);
  }
}

function idle(message) {
  runner?.terminate();
  runner = null;
  runButton.disabled = false;
  stopButton.disabled = true;
  status.textContent = message;
}

function finish(result) {
  flush();
  // A falha de execução vai junto da saída; os erros e avisos já estão nos diagnósticos.
  if (result.failure) {
    const failure = document.createElement("span");
    failure.className = "failure";
    failure.textContent = (output.textContent && !output.textContent.endsWith("\n") ? "\n" : "") + result.failure;
    output.append(failure);
  }
  const message = {
    ok: "Terminou.",
    errors: "O programa tem erros: veja os diagnósticos.",
    failed: `Terminou com falha (código ${result.exit}).`,
  }[result.status];
  idle(message);
}

function run() {
  runner?.terminate();
  output.textContent = "";
  buffer = "";
  status.textContent = "Rodando...";
  runButton.disabled = true;
  stopButton.disabled = false;
  runner = new Worker(workerUrl, { type: "module" });
  runner.onmessage = (event) => {
    if (event.data.kind === "out") append(event.data.text);
    else if (event.data.kind === "done") finish(event.data.result);
  };
  runner.onerror = (event) => {
    flush();
    idle(`O interpretador falhou no navegador: ${event.message}`);
  };
  runner.postMessage({ kind: "run", source: view.state.doc.toString(), input: input.value });
}

runButton.addEventListener("click", run);
stopButton.addEventListener("click", () => {
  flush();
  idle("Parado.");
});
view.contentDOM.addEventListener("keydown", (event) => {
  if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
    event.preventDefault();
    run();
  }
});

// Compartilhar: o código vai no endereço (#codigo=...), em base64url do UTF-8.
const encode = (text) => {
  let bin = "";
  for (const b of new TextEncoder().encode(text)) bin += String.fromCharCode(b);
  return btoa(bin).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
};
const decode = (text) => {
  const bin = atob(text.replaceAll("-", "+").replaceAll("_", "/"));
  return new TextDecoder().decode(Uint8Array.from(bin, (c) => c.charCodeAt(0)));
};

shareButton.addEventListener("click", async () => {
  const url = new URL(location.href);
  url.search = "";
  url.hash = `codigo=${encode(view.state.doc.toString())}`;
  history.replaceState(null, "", url);
  try {
    await navigator.clipboard.writeText(url.href);
    status.textContent = "O link com o código foi copiado.";
  } catch {
    status.textContent = "O link com o código está no endereço da página.";
  }
});

// Os exemplos.
for (const s of samples) {
  const option = document.createElement("option");
  option.value = s.name;
  option.textContent = s.title;
  select.append(option);
}
select.addEventListener("change", () => {
  const s = samples.find((x) => x.name === select.value);
  if (!s) return;
  setCode(s.code, s.input);
  const url = new URL(location.href);
  url.hash = "";
  url.search = `?poema=${s.name}`;
  history.replaceState(null, "", url);
  output.textContent = "";
  status.textContent = "";
});

// O que abre: o código do link, o poema pedido, o último texto, ou o primeiro exemplo.
const params = new URLSearchParams(location.search);
const shared = location.hash.startsWith("#codigo=") ? location.hash.slice(8) : null;
let stored = null;
try {
  stored = JSON.parse(localStorage.getItem(STORE) ?? "null");
} catch {}
const askedPoem = samples.find((s) => s.name === params.get("poema"));
if (shared) {
  try {
    setCode(decode(shared));
  } catch {
    setCode(samples[0].code, samples[0].input);
  }
} else if (askedPoem) {
  setCode(askedPoem.code, askedPoem.input);
  select.value = askedPoem.name;
} else if (stored?.code) {
  setCode(stored.code, stored.input ?? "");
} else {
  setCode(samples[0].code, samples[0].input);
  select.value = samples[0].name;
}
if (params.has("rodar")) run();
