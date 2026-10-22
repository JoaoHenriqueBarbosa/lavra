// Os testes de dentro do VS Code: abrir um programa da Lavra e conferir o que o servidor
// de linguagem entrega pelo editor.
import * as assert from "node:assert";
import { execFileSync } from "node:child_process";
import * as fs from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

async function until<T>(what: string, f: () => T | undefined | Promise<T | undefined>, ms = 30000): Promise<T> {
  const end = Date.now() + ms;
  for (;;) {
    const v = await f();
    if (v !== undefined) {
      return v;
    }
    if (Date.now() > end) {
      throw new Error(`não chegou: ${what}`);
    }
    await sleep(200);
  }
}

/** Uma sessão de depuração de verdade: para no ponto, confere a pilha e as variáveis pelo
 * adaptador, tira a captura parada (se pedida) e segue até o fim. */
async function debugSession(folder: string, shot: ((what: string) => string) | undefined): Promise<void> {
  const file = path.join(folder, "depurar.lav");
  const uri = vscode.Uri.file(file);
  const doc = await vscode.workspace.openTextDocument(uri);
  await vscode.window.showTextDocument(doc);
  const stopped: string[] = [];
  let terminated = false;
  const tracker = vscode.debug.registerDebugAdapterTrackerFactory("lavra", {
    createDebugAdapterTracker: () => ({
      onWillReceiveMessage: (m: unknown) => {
        if (process.env.LAVRA_DAP_LOG) {
          fs.appendFileSync(process.env.LAVRA_DAP_LOG, `-> ${JSON.stringify(m)}\n`);
        }
      },
      onDidSendMessage: (m: { type: string; event?: string; body?: { reason?: string } }) => {
        if (process.env.LAVRA_DAP_LOG) {
          fs.appendFileSync(process.env.LAVRA_DAP_LOG, `<- ${JSON.stringify(m)}\n`);
        }
        if (m.type === "event" && m.event === "stopped") {
          stopped.push(m.body?.reason ?? "");
        }
        if (m.type === "event" && m.event === "terminated") {
          terminated = true;
        }
      },
    }),
  });
  vscode.debug.addBreakpoints([new vscode.SourceBreakpoint(new vscode.Location(uri, new vscode.Position(6, 4)))]);
  // A vista de depuração aberta antes, como fica quando se depura: o VS Code só abre o
  // primeiro escopo sozinho se ela já está à mostra quando o programa para.
  await vscode.commands.executeCommand("workbench.view.debug");
  await sleep(1500);
  const ok = await vscode.debug.startDebugging(vscode.workspace.workspaceFolders![0], {
    type: "lavra",
    request: "launch",
    name: "Depurar o arquivo",
    program: file,
  });
  assert.ok(ok, "a depuração não começou");
  await until("a parada no ponto", () => (stopped.length > 0 ? stopped[0] : undefined));
  assert.strictEqual(stopped[0], "breakpoint");
  const session = vscode.debug.activeDebugSession!;
  const st = await session.customRequest("stackTrace", { threadId: 1 });
  assert.strictEqual(st.stackFrames[0].line, 7);
  const scopes = await session.customRequest("scopes", { frameId: st.stackFrames[0].id });
  const locals = scopes.scopes.find((s: { name: string }) => s.name === "Locais");
  const vars = await session.customRequest("variables", { variablesReference: locals.variablesReference });
  assert.deepStrictEqual([vars.variables[0].name, vars.variables[0].value], ["o saldo", "100"]);
  if (shot) {
    await sleep(2500);
    execFileSync("import", ["-window", "root", shot("depurador")]);
  }
  await session.customRequest("continue", { threadId: 1 });
  await until("o fim do programa", () => (terminated ? true : undefined));
  tracker.dispose();
}

/** A posse no editor: o hover no nome que entrega o valor diz a quem, e o nome leva o
 * modificador `moved` (em itálico). Com a captura, o hover aberto sobre ele. */
async function ownership(folder: string, shot: string | undefined): Promise<void> {
  const uri = vscode.Uri.file(path.join(folder, "posse.lav"));
  const doc = await vscode.workspace.openTextDocument(uri);
  const editor = await vscode.window.showTextDocument(doc);
  const given = new vscode.Position(7, 16);
  const text = await until("o hover da posse", async () => {
    const hovers = await vscode.commands.executeCommand<vscode.Hover[]>("vscode.executeHoverProvider", uri, given);
    const t = hovers.map((h) => h.contents.map((c) => (c as vscode.MarkdownString).value).join("")).join("");
    return t.includes("entregues") ? t : undefined;
  });
  assert.ok(text.includes('As cartas são entregues a "arquivar" aqui: daqui em diante, não se usam mais.'), text);
  const legend = await vscode.commands.executeCommand<vscode.SemanticTokensLegend>("vscode.provideDocumentSemanticTokensLegend", uri);
  const tokens = await vscode.commands.executeCommand<vscode.SemanticTokens>("vscode.provideDocumentSemanticTokens", uri);
  const moved = 1 << legend.tokenModifiers.indexOf("moved");
  const at: Array<[number, number, number]> = [];
  let line = 0;
  let col = 0;
  for (let i = 0; i < tokens.data.length; i += 5) {
    if (tokens.data[i] > 0) {
      col = 0;
    }
    line += tokens.data[i];
    col += tokens.data[i + 1];
    at.push([line, col, tokens.data[i + 4]]);
  }
  const cartas = at.find(([l, c]) => l === 7 && c === 15);
  assert.ok(cartas && (cartas[2] & moved) !== 0, `"cartas" sem o modificador moved: ${JSON.stringify(at)}`);
  if (shot) {
    editor.selection = new vscode.Selection(given, given);
    await vscode.commands.executeCommand("editor.action.showHover");
    await sleep(2500);
    execFileSync("import", ["-window", "root", shot]);
    await vscode.commands.executeCommand("editor.action.hideHover");
  }
}

export async function run(): Promise<void> {
  const folder = vscode.workspace.workspaceFolders![0].uri.fsPath;
  const uri = vscode.Uri.file(path.join(folder, "conta.lav"));
  const doc = await vscode.workspace.openTextDocument(uri);
  let editor = await vscode.window.showTextDocument(doc);
  assert.strictEqual(doc.languageId, "lavra");

  // Diagnósticos ao abrir: o nome errado, com o código e o link da explicação.
  const server = vscode.workspace.getConfiguration("lavra").get<string>("server.path") ?? "";
  const ext = vscode.extensions.all.find((e) => e.id.endsWith(".lavra"));
  const context = `servidor "${server}" (existe: ${fs.existsSync(server)}), extensão ativa: ${ext?.isActive}`;
  const diags = await until(`diagnósticos; ${context}`, () => {
    const d = vscode.languages.getDiagnostics(uri);
    return d.length > 0 ? d : undefined;
  });
  assert.strictEqual(diags.length, 1);
  const code = diags[0].code as { value: string; target: vscode.Uri };
  assert.strictEqual(code.value, "L0334");
  assert.ok(code.target.toString().endsWith("L0334.md"));

  // Hover no "saldo" de "Escrevo o saldo.".
  const hovers = await vscode.commands.executeCommand<vscode.Hover[]>("vscode.executeHoverProvider", uri, new vscode.Position(6, 15));
  const text = hovers.map((h) => h.contents.map((c) => (c as vscode.MarkdownString).value).join("")).join("");
  assert.ok(text.includes("variável, um inteiro"), text);

  // Tokens semânticos: há verbos, parâmetros e variáveis.
  const legend = await vscode.commands.executeCommand<vscode.SemanticTokensLegend>("vscode.provideDocumentSemanticTokensLegend", uri);
  const tokens = await vscode.commands.executeCommand<vscode.SemanticTokens>("vscode.provideDocumentSemanticTokens", uri);
  const kinds = new Set<string>();
  for (let i = 3; i < tokens.data.length; i += 5) {
    kinds.add(legend.tokenTypes[tokens.data[i]]);
  }
  for (const k of ["function", "parameter", "variable"]) {
    assert.ok(kinds.has(k), `falta o token ${k}: ${[...kinds]}`);
  }

  // Completar no começo da oração: o verbo do programa com a regência.
  const list = await vscode.commands.executeCommand<vscode.CompletionList>("vscode.executeCompletionItemProvider", uri, new vscode.Position(5, 4));
  const rendo = list.items.find((i) => i.label === "Rendo");
  assert.ok(rendo, `sem "Rendo" no completar: ${list.items.map((i) => i.label).join(", ")}`);

  // Ajuda de assinatura depois do verbo, e os inlay hints: o tipo da declaração sem aposto
  // e o argumento confiado.
  const help = await vscode.commands.executeCommand<vscode.SignatureHelp>("vscode.executeSignatureHelpProvider", uri, new vscode.Position(5, 10));
  assert.strictEqual(help?.signatures[0]?.label, "rendo QUANTIA");
  const hints = await vscode.commands.executeCommand<vscode.InlayHint[]>("vscode.executeInlayHintProvider", uri, new vscode.Range(0, 0, 8, 0));
  const labels = hints.map((h) => (typeof h.label === "string" ? h.label : h.label.map((p) => p.value).join("")));
  for (const want of [", um inteiro,", "confiado:"]) {
    assert.ok(labels.includes(want), `falta o inlay hint "${want}": ${labels.join(" | ")}`);
  }

  // Rename com morfologia: "o saldo" vira "a reserva", e o artigo vai junto.
  const renamed = await vscode.commands.executeCommand<vscode.WorkspaceEdit>(
    "vscode.executeDocumentRenameProvider",
    uri,
    new vscode.Position(6, 15),
    "a reserva",
  );
  const edits = renamed.get(uri).map((e) => e.newText);
  assert.ok(edits.includes("reserva") && edits.includes("A"), `rename: ${edits.join(" | ")}`);

  // Definição: do uso à declaração.
  const defs = await vscode.commands.executeCommand<vscode.Location[]>("vscode.executeDefinitionProvider", uri, new vscode.Position(6, 15));
  assert.strictEqual(defs[0].range.start.line, 4);

  // A posse: o hover e o itálico no nome que entrega o valor.
  const shotOf = process.env.LAVRA_SHOT;
  await ownership(folder, shotOf ? shotOf.replace(/\.png$/, "-posse.png") : undefined);

  // A depuração, num programa sem erro (o de cima tem um, de propósito).
  await debugSession(folder, shotOf ? (what) => shotOf.replace(/\.png$/, `-${what}.png`) : undefined);
  editor = await vscode.window.showTextDocument(doc);

  // As capturas da janela, quando pedidas: o hover com o realce semântico; o campo de
  // rename; uma oração nova com o completar aberto e os inlay hints no texto; a ajuda de
  // assinatura.
  const shot = process.env.LAVRA_SHOT;
  if (shot) {
    const named = (what: string) => shot.replace(/\.png$/, `-${what}.png`);
    editor.selection = new vscode.Selection(6, 15, 6, 15);
    await vscode.commands.executeCommand("editor.action.showHover");
    await sleep(2500);
    execFileSync("import", ["-window", "root", shot]);
    await vscode.commands.executeCommand("editor.action.hideHover");
    // O campo de rename, com o nome proposto com o artigo.
    // O comando só termina quando o campo fecha: não se espera por ele.
    editor.selection = new vscode.Selection(6, 15, 6, 15);
    void vscode.commands.executeCommand("editor.action.rename");
    await sleep(2500);
    execFileSync("import", ["-window", "root", named("rename")]);
    await vscode.commands.executeCommand("cancelRenameInput");
    await sleep(500);
    await editor.edit((e) => e.insert(new vscode.Position(5, 18), "\n    "));
    editor.selection = new vscode.Selection(6, 4, 6, 4);
    await vscode.commands.executeCommand("editor.action.triggerSuggest");
    await sleep(2500);
    execFileSync("import", ["-window", "root", named("completar")]);
    await vscode.commands.executeCommand("hideSuggestWidget");
    await editor.edit((e) => e.insert(new vscode.Position(6, 4), "Rendo "));
    editor.selection = new vscode.Selection(6, 10, 6, 10);
    await vscode.commands.executeCommand("editor.action.triggerParameterHints");
    await sleep(2500);
    execFileSync("import", ["-window", "root", named("assinatura")]);
  }
}
