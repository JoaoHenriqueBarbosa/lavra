// Roda os testes de integração num VS Code de verdade (baixado pelo @vscode/test-electron),
// com a extensão em desenvolvimento e a pasta de exemplo aberta.
import * as fs from "node:fs";
import * as path from "node:path";
import { runTests } from "@vscode/test-electron";

async function main(): Promise<void> {
  const root = path.resolve(__dirname, "..");
  // A pasta de exemplo usa o lavrac do repositório (LAVRA_SERVER muda), com o realce
  // semântico ligado.
  const fixture = path.join(root, "tests", "integration", "fixture");
  const server = process.env.LAVRA_SERVER ?? path.resolve(root, "..", "..", "target", "release", "lavrac");
  fs.mkdirSync(path.join(fixture, ".vscode"), { recursive: true });
  fs.writeFileSync(
    path.join(fixture, ".vscode", "settings.json"),
    JSON.stringify({ "editor.semanticHighlighting.enabled": true, "lavra.server.path": server }, null, 2),
  );
  await runTests({
    extensionDevelopmentPath: root,
    extensionTestsPath: path.join(__dirname, "suite.js"),
    launchArgs: [path.join(root, "tests", "integration", "fixture"), "--disable-extensions", "--disable-workspace-trust", "--disable-gpu", "--no-sandbox"],
  });
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
