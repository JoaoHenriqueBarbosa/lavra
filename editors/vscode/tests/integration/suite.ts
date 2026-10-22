// Os testes de dentro do VS Code: abrir um programa da Lavra e conferir o que o servidor
// de linguagem entrega pelo editor.
import * as assert from "node:assert";
import { execFileSync } from "node:child_process";
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
  const diags = await until("diagnósticos", () => {
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

  // Definição: do uso à declaração.
  const defs = await vscode.commands.executeCommand<vscode.Location[]>("vscode.executeDefinitionProvider", uri, new vscode.Position(6, 15));
  assert.strictEqual(defs[0].range.start.line, 4);

  // A captura da janela, com o realce semântico aplicado, quando pedida.
  const shot = process.env.LAVRA_SHOT;
  if (shot) {
    editor.selection = new vscode.Selection(6, 15, 6, 15);
    await vscode.commands.executeCommand("editor.action.showHover");
    await sleep(2500);
    execFileSync("import", ["-window", "root", shot]);
  }
}
