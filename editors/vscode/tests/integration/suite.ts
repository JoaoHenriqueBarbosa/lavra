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

export async function run(): Promise<void> {
  const folder = vscode.workspace.workspaceFolders![0].uri.fsPath;
  const uri = vscode.Uri.file(path.join(folder, "conta.lav"));
  const doc = await vscode.workspace.openTextDocument(uri);
  const editor = await vscode.window.showTextDocument(doc);
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
