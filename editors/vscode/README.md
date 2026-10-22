# Lavra para VS Code

Suporte à [Lavra](https://github.com/JoaoHenriqueBarbosa/lavra), a linguagem compilada
em que cada instrução é uma oração em português.

## O que faz

- **Diagnósticos ao digitar**, do próprio compilador: todos os erros e avisos de uma vez,
  com o código (`L0334`) e o link para a explicação, a dica e os trechos relacionados.
- **Realce semântico**: verbos do programa (também nos predicados), parâmetros emprestados
  e confiados, o que é fixo e o que muda, campos, tipos e casos, por cima da gramática
  TextMate.
- **Hover**: o tipo por extenso e o modo de posse de cada nome, a regência de cada verbo, e
  a explicação das palavras da língua.
- **Navegação**: ir para a definição, referências, destaque das ocorrências, símbolos do
  arquivo e do workspace.
- **Rodar, compilar e verificar**: botão de rodar no título do editor, comandos na paleta
  ("Lavra: Rodar o arquivo") e tarefas com os erros no painel Problemas.
- **Snippets** dos blocos: `para começar`, `se`, `para cada`, `conforme`, `registro`.
- **Edição**: comentário com `--`, pares e aspas, recuo depois de `:`, `Senão` volta o
  recuo, dobra por indentação.

O servidor de linguagem é o próprio compilador (`lavrac lsp`). A extensão procura o
`lavrac` no setting `lavra.server.path`, depois o que vem dentro dela, depois o do PATH.

## Desenvolvimento

```bash
npm install
npm test                       # escopos da gramática (tests/syntax) e snapshots dos samples
npm run compile                # confere os tipos e empacota o cliente em dist/
npm run test:integration       # num VS Code de verdade (precisa de um display; no CI, xvfb)
npm run package                # gera lavra.vsix
code --install-extension lavra.vsix
```

As palavras da gramática vêm do léxico do compilador (`editors/lexicon.json`); as regras
ficam em `syntaxes/build.py`. Não edite o JSON da gramática à mão.
