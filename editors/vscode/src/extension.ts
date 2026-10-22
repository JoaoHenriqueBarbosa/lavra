// Cliente do VS Code para a Lavra: liga o servidor de linguagem (`lavrac lsp`) e o
// adaptador de depuração (`lavrac dap`), e oferece os comandos de rodar, depurar, compilar e
// verificar, e tarefas com os erros no painel Problemas.

import * as fs from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";
import { LanguageClient, type LanguageClientOptions, type ServerOptions, TransportKind } from "vscode-languageclient/node";

let client: LanguageClient | undefined;

/** O esquema dos endereços das bibliotecas ("lavra-library:/textos.lav"): a definição de um
 * verbo de biblioteca abre o texto dela, só de leitura, pedido ao servidor. */
const LIBRARY_SCHEME = "lavra-library";

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
    // Só os arquivos do programa: o texto de uma biblioteca aberto pela definição não é
    // programa e não se analisa.
    documentSelector: [
      { scheme: "file", language: "lavra" },
      { scheme: "untitled", language: "lavra" },
    ],
    initializationOptions: { libraryDocuments: true },
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
  if (!editor || editor.document.languageId !== "lavra" || editor.document.uri.scheme === LIBRARY_SCHEME) {
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

/** O adaptador de depuração é o próprio `lavrac`, o mesmo do servidor de linguagem. */
function adapterFactory(context: vscode.ExtensionContext): vscode.DebugAdapterDescriptorFactory {
  return {
    createDebugAdapterDescriptor: () => new vscode.DebugAdapterExecutable(serverPath(context), ["dap"]),
  };
}

/** A configuração de depurar o arquivo aberto. */
function debugFile(file: string): vscode.DebugConfiguration {
  return { type: "lavra", request: "launch", name: "Depurar o arquivo", program: file };
}

/** F5 sem launch.json depura o arquivo .lav aberto; o arquivo é salvo antes. */
const configurationProvider: vscode.DebugConfigurationProvider = {
  async resolveDebugConfiguration(_folder, config) {
    const editor = vscode.window.activeTextEditor;
    const open = editor?.document.languageId === "lavra" ? editor.document : undefined;
    if (!config.type && !config.request && !config.name) {
      if (!open) {
        return undefined;
      }
      config = debugFile(open.fileName);
    }
    if (!config.program && open) {
      config.program = open.fileName;
    }
    if (!config.program) {
      vscode.window.showWarningMessage("Diga o \"program\": o arquivo .lav a depurar.");
      return undefined;
    }
    const doc = vscode.workspace.textDocuments.find((d) => d.fileName === config.program);
    if (doc?.isDirty) {
      await doc.save();
    }
    return config;
  },
};

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  context.subscriptions.push(
    vscode.debug.registerDebugAdapterDescriptorFactory("lavra", adapterFactory(context)),
    vscode.debug.registerDebugConfigurationProvider("lavra", configurationProvider),
    vscode.commands.registerCommand("lavra.debug", async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor || editor.document.languageId !== "lavra" || editor.document.uri.scheme === LIBRARY_SCHEME) {
        vscode.window.showWarningMessage("Abra um arquivo .lav primeiro.");
        return;
      }
      await editor.document.save();
      await vscode.debug.startDebugging(vscode.workspace.getWorkspaceFolder(editor.document.uri), debugFile(editor.document.fileName));
    }),
    vscode.commands.registerCommand("lavra.run", () => inTerminal(context, "run")),
    vscode.commands.registerCommand("lavra.build", () => inTerminal(context, "build")),
    vscode.commands.registerCommand("lavra.check", () => inTerminal(context, "check")),
    vscode.commands.registerCommand("lavra.restartServer", async () => {
      await client?.stop();
      await startClient(context);
    }),
    vscode.tasks.registerTaskProvider("lavra", taskProvider(context)),
    vscode.workspace.registerTextDocumentContentProvider(LIBRARY_SCHEME, {
      provideTextDocumentContent: async (uri) => {
        const answer = await client?.sendRequest<{ text: string }>("lavra/library", { uri: uri.toString() });
        return answer?.text ?? "";
      },
    }),
  );
  await startClient(context);
}

export async function deactivate(): Promise<void> {
  await client?.stop();
}
