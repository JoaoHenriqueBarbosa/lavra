// O interpretador da Lavra (wasm) fora da página: conferir enquanto se escreve, e rodar.
// Um laço sem fim fica preso aqui, e a página continua viva: o botão de parar encerra o
// worker.
import init, { check, run } from "./pkg/lavra_playground.js";

const ready = init();

self.onmessage = async (event) => {
  await ready;
  const msg = event.data;
  if (msg.kind === "check") {
    self.postMessage({ kind: "checked", id: msg.id, diagnostics: JSON.parse(check(msg.source)) });
    return;
  }
  let result;
  try {
    result = JSON.parse(run(msg.source, msg.input, (text) => self.postMessage({ kind: "out", text })));
  } catch (err) {
    // O que o interpretador não alcança relatar: a memória ou a pilha do navegador acabou.
    result = { status: "failed", exit: 101, stderr: `O navegador não deu conta deste programa: ${err?.message ?? err}\n` };
  }
  self.postMessage({ kind: "done", result });
};
