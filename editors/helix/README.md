# A Lavra no Helix

Realce, recuo, objetos de texto e a estrutura pela gramática tree-sitter da Lavra; o
servidor de linguagem e o depurador pelo próprio `lavrac`. Precisa do Helix 25.07 ou mais
novo: é a partir dele que, entre dois padrões de realce sobre o mesmo trecho, vence o que vem
depois, como nas queries daqui.

## Instalar

1. Instale o `lavrac` no PATH, pela [página da última versão](https://github.com/JoaoHenriqueBarbosa/lavra/releases/latest).
2. Junte o [`languages.toml`](languages.toml) daqui ao seu `~/.config/helix/languages.toml`
   (no Windows, `%AppData%\helix\languages.toml`).
3. Copie as queries:

   ```bash
   mkdir -p ~/.config/helix/runtime/queries
   cp -r runtime/queries/lavra ~/.config/helix/runtime/queries/
   ```

4. Baixe e compile a gramática. O Helix usa o git e o compilador de C++ do sistema:

   ```bash
   hx --grammar fetch
   hx --grammar build
   ```

5. Confira: `hx --health lavra` mostra o `lavrac` como servidor e depurador, e o parser e as
   queries de realce, objetos de texto e recuo com ✓.

A gramática vem do repositório da Lavra no commit do `rev`, que acompanha a gramática de
`editors/tree-sitter-lavra/src` (o CI confere). Para usar a de uma cópia local, troque o
`source` por `{ path = "/caminho/para/lavra/editors/tree-sitter-lavra" }` e crie antes a pasta
`~/.config/helix/runtime/grammars`.

## O que vem com o servidor

- Diagnósticos ao digitar, com as correções seguras como ações de código, hover com o tipo e a
  posse, definição, referências, renomear, símbolos, completar, ajuda de assinatura e
  formatação.
- Inlay hints: o tipo das declarações sem aposto, o modo dos argumentos e o tipo de
  "o resultado". O Helix só os mostra com

  ```toml
  [editor.lsp]
  display-inlay-hints = true
  ```

  no `config.toml`, e cada tipo de dica se desliga em `[language-server.lavrac.config]`, no
  `languages.toml` (por exemplo `inlayHints = { types = false }`).
- O depurador (`lavrac dap`): `:debug-start`, ou o menu de depuração, com "depurar o arquivo"
  e "depurar o arquivo, com a entrada de um arquivo".

## O que o Helix não tem

- **Tokens semânticos.** O Helix não pede tokens semânticos ao servidor, então o realce é só o
  da gramática: o verbo do programa no meio da oração, o parâmetro emprestado e o nome que
  entrega o valor (em itálico no VS Code) não ganham cor própria.
- **O texto das bibliotecas.** A definição de um verbo de biblioteca ("Divido", de
  `Uso a biblioteca de textos.`) fica vazia, porque o Helix só abre arquivos; o hover diz de
  que biblioteca o verbo vem.

## Conferir

`editors/tree-sitter-lavra/scripts/helix-health.sh` faz tudo isso numa configuração
temporária, sem tocar na sua: compila a gramática da cópia local, roda o `hx --health lavra`,
abre um programa do corpus num tmux, usa o recuo e os objetos de texto, e confere que o log do
Helix não tem erro de query.
