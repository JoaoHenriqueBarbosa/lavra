# A Lavra no Zed

Uma extensão do Zed: o realce, o recuo, os pares, o esboço e os objetos de texto pela
gramática tree-sitter da Lavra, e o servidor de linguagem pelo próprio `lavrac`.

## Instalar

1. Instale o `lavrac` no PATH, pela [página da última versão](https://github.com/JoaoHenriqueBarbosa/lavra/releases/latest),
   ou indique o caminho dele nas configurações (abaixo).
2. Instale o Rust pelo [rustup](https://rustup.rs): o Zed compila a extensão para
   `wasm32-wasip2` e baixa sozinho o wasi-sdk, que compila a gramática.
3. Na página de extensões, **Install Dev Extension** (ou a ação `zed: install dev extension`),
   e escolha esta pasta, `editors/zed`.

A gramática vem do repositório da Lavra no commit do `rev` em [`extension.toml`](extension.toml),
que acompanha a gramática de `editors/tree-sitter-lavra/src` (o CI confere). Para usar a de
uma cópia local, troque o `repository` por `file:///caminho/para/lavra` e ponha no `rev` um
commit dessa cópia.

## Configurações

No `settings.json` do Zed:

```json
{
  "lsp": {
    "lavrac": {
      "binary": { "path": "/caminho/do/lavrac" },
      "initialization_options": { "inlayHints": { "types": false } }
    }
  },
  "inlay_hints": { "enabled": true },
  "languages": {
    "Lavra": { "semantic_tokens": "combined" }
  }
}
```

- **O servidor** (`lsp.lavrac`): o `binary.path` troca o `lavrac` do PATH; as
  `initialization_options` vão ao servidor ao iniciar, e os `settings` a cada mudança.
- **Inlay hints.** O tipo das declarações sem aposto (`types`), o modo dos argumentos (`modes`)
  e o tipo de "o resultado" (`result`) começam ligados no servidor; o Zed só os mostra com
  `inlay_hints.enabled`.
- **Tokens semânticos.** O Zed não os pede por padrão. Com `"semantic_tokens": "combined"`, as
  cores do compilador (o verbo do programa no meio da oração, parâmetros, campos, tipos e
  casos) vêm por cima do realce da gramática, e o nome no ponto em que entrega o valor (o
  modificador `moved`) sai em itálico, com a cor que já tem, pela regra da extensão em
  [`languages/lavra/semantic_token_rules.json`](languages/lavra/semantic_token_rules.json). Uma
  regra sua em `global_lsp_settings.semantic_token_rules` vale por cima dela.

## O que o Zed não tem

- **O texto das bibliotecas.** A definição de um verbo de biblioteca ("Divido", de
  `Uso a biblioteca de textos.`) fica vazia, porque o Zed só abre arquivos; o hover diz de que
  biblioteca o verbo vem.
- **O depurador.** No Zed, um adaptador de depuração é código da extensão; esta ainda não
  registra o `lavrac dap`.

## Conferir

A extensão compila com:

```bash
rustup target add wasm32-wasip2
cargo build --release --target wasm32-wasip2
```

e as queries daqui compilam contra a gramática no `npm run test:queries` de
`editors/tree-sitter-lavra`.
