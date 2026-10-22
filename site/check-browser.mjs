// O playground num navegador de verdade: serve site/dist por HTTP (ou usa o site publicado,
// com --url), abre o Chromium sem janela pelo protocolo de depuração (CDP), roda cada poema
// no playground e confere que a saída da página é a esperada (samples/*.stdout). Com --shot
// arquivo.png, tira a captura do primeiro poema rodado. O navegador vem de CHROME, ou é o
// chromium/google-chrome do PATH.
import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs";
import http from "node:http";
import path from "node:path";

const here = import.meta.dirname;
const root = path.resolve(here, "..");
const dist = path.join(here, "dist");
const args = process.argv.slice(2);
const shot = args.includes("--shot") ? path.resolve(args[args.indexOf("--shot") + 1]) : null;
// Com --url, confere o site publicado no lugar da cópia local.
const remote = args.includes("--url") ? args[args.indexOf("--url") + 1].replace(/\/$/, "") : null;
const only = args.filter((a, i) => !a.startsWith("--") && !["--shot", "--url"].includes(args[i - 1]));

const TYPES = { ".html": "text/html; charset=utf-8", ".js": "text/javascript", ".css": "text/css", ".wasm": "application/wasm", ".json": "application/json" };
const server = http.createServer((req, res) => {
  const url = new URL(req.url, "http://localhost");
  const file = path.join(dist, decodeURIComponent(url.pathname));
  if (!file.startsWith(dist) || !fs.existsSync(file) || fs.statSync(file).isDirectory()) {
    res.writeHead(404).end();
    return;
  }
  res.writeHead(200, { "content-type": TYPES[path.extname(file)] ?? "application/octet-stream" });
  fs.createReadStream(file).pipe(res);
});
await new Promise((r) => server.listen(0, "127.0.0.1", r));
const base = remote ?? `http://127.0.0.1:${server.address().port}`;

const onPath = (c) => {
  try {
    execFileSync("sh", ["-c", `command -v ${c}`], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
};
const chrome = process.env.CHROME ?? ["chromium", "chromium-browser", "google-chrome", "google-chrome-stable"].find(onPath);
if (!chrome) {
  console.error("não achei o Chromium nem o Chrome: diga qual em CHROME");
  process.exit(2);
}
const profile = fs.mkdtempSync(path.join(fs.realpathSync("/tmp"), "lavra-chrome-"));
const browser = spawn(chrome, ["--headless=new", "--remote-debugging-port=0", `--user-data-dir=${profile}`, "--window-size=1360,900", "--no-first-run", "--no-sandbox", "about:blank"], {
  stdio: ["ignore", "ignore", "pipe"],
});
const wsUrl = await new Promise((resolve, reject) => {
  let err = "";
  browser.stderr.on("data", (d) => {
    err += d;
    const m = err.match(/DevTools listening on (ws:\/\/\S+)/);
    if (m) resolve(m[1]);
  });
  browser.on("exit", () => reject(new Error(`o navegador saiu: ${err}`)));
});

// Uma conexão CDP com a página.
const port = new URL(wsUrl).port;
const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
const ws = new WebSocket(targets.find((t) => t.type === "page").webSocketDebuggerUrl);
await new Promise((r) => ws.addEventListener("open", r, { once: true }));
let next = 0;
const calls = new Map();
ws.addEventListener("message", (e) => {
  const msg = JSON.parse(e.data);
  if (msg.id && calls.has(msg.id)) {
    calls.get(msg.id)(msg);
    calls.delete(msg.id);
  }
});
const send = (method, params = {}) =>
  new Promise((resolve, reject) => {
    next += 1;
    calls.set(next, (msg) => (msg.error ? reject(new Error(msg.error.message)) : resolve(msg.result)));
    ws.send(JSON.stringify({ id: next, method, params }));
  });
const evaluate = async (expression) => (await send("Runtime.evaluate", { expression, returnByValue: true })).result.value;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

await send("Page.enable");
await send("Runtime.enable");

const samples = fs
  .readdirSync(path.join(root, "samples"))
  .filter((f) => f.endsWith(".lav") && fs.existsSync(path.join(root, "samples", f.replace(/\.lav$/, ".stdout"))))
  .map((f) => f.slice(0, -4))
  .filter((name) => !only.length || only.includes(name))
  .sort();

let failures = 0;
let shotTaken = false;
for (const name of samples) {
  const expected = fs.readFileSync(path.join(root, "samples", `${name}.stdout`), "utf8");
  await send("Page.navigate", { url: `${base}/playground.html?poema=${name}&rodar` });
  let state = "";
  const end = Date.now() + 30_000;
  while (Date.now() < end) {
    await sleep(150);
    state = (await evaluate("document.querySelector('#status')?.textContent ?? ''").catch(() => "")) ?? "";
    if (state && state !== "Rodando...") break;
  }
  // O que o programa escreveu, sem a falha de execução que a página põe depois.
  const text = await evaluate(
    "[...document.querySelector('#saida').childNodes].filter((n) => !n.classList?.contains('failure')).map((n) => n.textContent).join('')",
  );
  const ok = text === expected;
  console.log(`${ok ? "igual" : "DIFERENTE"}  ${name}  (${state})`);
  if (!ok) {
    failures += 1;
    console.log(`  esperava ${JSON.stringify(expected.slice(0, 160))}\n  veio     ${JSON.stringify(text.slice(0, 160))}`);
  }
  if (shot && !shotTaken) {
    await sleep(400);
    const { data } = await send("Page.captureScreenshot", { format: "png" });
    fs.writeFileSync(shot, Buffer.from(data, "base64"));
    shotTaken = true;
  }
}

ws.close();
browser.kill();
server.close();
fs.rmSync(profile, { recursive: true, force: true });
console.log(`${samples.length - failures} de ${samples.length} poemas com a saída esperada no navegador`);
process.exit(failures ? 1 : 0);
