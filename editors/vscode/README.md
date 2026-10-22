# Lavra para VS Code

Suporte à [Lavra](https://github.com/JoaoHenriqueBarbosa/lavra), a linguagem compilada
em que cada instrução é uma oração em português.

## O que faz

- **Diagnósticos ao digitar**, do próprio compilador: todos os erros e avisos de uma vez,
  com o código (`L0334`) e o link para a explicação, a dica e os trechos relacionados.
- **Realce semântico**: verbos do programa (também nos predicados), parâmetros emprestados
  e confiados, o que é fixo e o que muda, campos, tipos e casos, por cima da gramática
  TextMate. O nome que entrega o valor ("Arquivo a carta.") sai em itálico: dali em diante,
  ele não se usa mais.
- **Hover**: o tipo por extenso e o modo de posse de cada nome, a quem o valor é entregue
  ali, a regência de cada verbo, e a explicação das palavras da língua.
- **Navegação**: ir para a definição, referências, destaque das ocorrências, símbolos do
  arquivo e do workspace.
- **Completar** guiado pelo próprio parser: as palavras que cabem ali, os verbos do
  programa com a regência como snippet, os nomes visíveis do gênero e do número do artigo
  que você escreveu, os campos e os tipos.
- **Ajuda de assinatura**: a regência do verbo da oração, com o parâmetro ativo.
- **Inlay hints**: o tipo das declarações sem aposto, "confiado:" e "emprestado:" antes dos
  argumentos, e o tipo de "o resultado".
- **Ações rápidas**: as correções que o compilador já sabe fazer (contração, concordância,
  ponto final, acento...).
- **Formatação** (`lavrac fmt`): recuo e espaços, sem nunca mexer em quebra de linha, porque
  os poemas também são programas.
- **Rename com morfologia** (F2): cada uso na forma em que foi escrito ("Rendo", "render",
  "exceder", "excederem"); o artigo e a contração acompanham o gênero do nome novo ("do
  saldo" vira "da reserva"), e o que concorda com ele vai junto. O rename é conferido pelo
  compilador e recusado, com o motivo, se deixaria erro.
- **Corrigir tudo**: as correções seguras do arquivo numa ação só (também ao salvar, com
  `"editor.codeActionsOnSave": {"source.fixAll": "explicit"}`).
- **Depurar** (F5, sem launch.json): pontos de parada, próxima linha, entrar e sair de
  função, pausa; a pilha com a linha de cada chamada; parâmetros, locais e constantes, com
  listas, registros e casos abrindo nas partes; o valor no hover; parada nos erros de
  execução. O programa que lê a entrada roda no terminal integrado; os outros, no console
  de depuração (com `"stdin"` para dar a entrada por arquivo).
- **Rodar, compilar e verificar**: botões de rodar e depurar no título do editor, comandos
  na paleta ("Lavra: Rodar o arquivo") e tarefas com os erros no painel Problemas.
- **Snippets** dos blocos: `para começar`, `se`, `para cada`, `conforme`, `registro`.
- **Edição**: comentário com `--`, pares e aspas, recuo depois de `:`, `Senão` volta o
  recuo; dobra dos blocos e dos comentários, e seleção crescente (palavra, oração, bloco).

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
