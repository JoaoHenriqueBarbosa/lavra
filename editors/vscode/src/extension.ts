// Cliente do VS Code para a Lavra: liga o servidor de linguagem (`lavrac lsp`) e oferece
// os comandos de rodar, compilar e verificar, e tarefas com os erros no painel Problemas.

import * as fs from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";
import { LanguageClient, type LanguageClientOptions, type ServerOptions, TransportKind } from "vscode-languageclient/node";

let client: LanguageClient | undefined;

/** O `lavrac`: o do setting, o embutido na extensão, ou o do PATH. */
export function serverPath(context: vscode.ExtensionContext): string {
  const configured = vscode.workspace.getConfiguration("lavra").get<string>("server.path");
  if (configured) {
    return configured;
  }
  const bundled = context.asAbsolutePath(path.join("server", process.platform === "win32" ? "lavrac.exe" : "lavrac"));
  return fs.existsSync(bundled) ? bundled : "lavrac";
}

async function startClient(context: vscode.ExtensionContext): Promise<void> {
  const command = serverPath(context);
  const serverOptions: ServerOptions = { command, args: ["lsp"], transport: TransportKind.stdio };
  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ language: "lavra" }],
    outputChannelName: "Lavra",
  };
  client = new LanguageClient("lavra", "Lavra", serverOptions, clientOptions);
  try {
    await client.start();
  } catch (e) {
    const pick = await vscode.window.showErrorMessage(
      `Não consegui iniciar o servidor da Lavra (${command}). Instale o lavrac ou diga o caminho no setting "lavra.server.path".`,
      "Abrir os settings",
    );
    if (pick) {
      await vscode.commands.executeCommand("workbench.action.openSettings", "lavra.server.path");
    }
    throw e;
  }
}

function quote(s: string): string {
  return /^[\w./-]+$/.test(s) ? s : `"${s.replace(/(["\\$`])/g, "\\$1")}"`;
}

/** Roda `lavrac <comando>` no arquivo aberto, num terminal chamado "Lavra". */
async function inTerminal(context: vscode.ExtensionContext, command: "run" | "build" | "check"): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.document.languageId !== "lavra") {
    vscode.window.showWarningMessage("Abra um arquivo .lav primeiro.");
    return;
  }
  await editor.document.save();
  const terminal = vscode.window.terminals.find((t) => t.name === "Lavra") ?? vscode.window.createTerminal("Lavra");
  terminal.show();
  terminal.sendText(`${quote(serverPath(context))} ${command} ${quote(editor.document.fileName)}`);
}

const TASKS: { command: "check" | "run" | "build"; label: string }[] = [
  { command: "check", label: "verificar" },
  { command: "run", label: "rodar" },
  { command: "build", label: "compilar para nativo" },
];

/** Tarefas do arquivo aberto, com os erros e avisos do `lavrac` no painel Problemas. */
function taskProvider(context: vscode.ExtensionContext): vscode.TaskProvider {
  const make = (command: string, label: string, file: string, folder: vscode.WorkspaceFolder | vscode.TaskScope) =>
    new vscode.Task(
      { type: "lavra", command, file },
      folder,
      `${label} ${path.basename(file)}`,
      "lavra",
      new vscode.ShellExecution(serverPath(context), [command, file]),
      ["$lavrac", "$lavrac-aviso"],
    );
  return {
    provideTasks() {
      const doc = vscode.window.activeTextEditor?.document;
      if (!doc || doc.languageId !== "lavra") {
        return [];
      }
      const folder = vscode.workspace.getWorkspaceFolder(doc.uri) ?? vscode.TaskScope.Workspace;
      return TASKS.map((t) => make(t.command, t.label, doc.fileName, folder));
    },
    resolveTask(task) {
      const def = task.definition as { command?: string; file?: string };
      const file = def.file ?? vscode.window.activeTextEditor?.document.fileName;
      if (!def.command || !file) {
        return undefined;
      }
      const label = TASKS.find((t) => t.command === def.command)?.label ?? def.command;
      return make(def.command, label, file, task.scope ?? vscode.TaskScope.Workspace);
    },
  };
}

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  context.subscriptions.push(
    vscode.commands.registerCommand("lavra.run", () => inTerminal(context, "run")),
    vscode.commands.registerCommand("lavra.build", () => inTerminal(context, "build")),
    vscode.commands.registerCommand("lavra.check", () => inTerminal(context, "check")),
    vscode.commands.registerCommand("lavra.restartServer", async () => {
      await client?.stop();
      await startClient(context);
    }),
    vscode.tasks.registerTaskProvider("lavra", taskProvider(context)),
  );
  await startClient(context);
}

export async function deactivate(): Promise<void> {
  await client?.stop();
}
