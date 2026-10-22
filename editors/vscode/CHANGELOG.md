# Mudanças

## 0.7.0

- A posse no editor: o nome que entrega o valor sai em itálico (modificador semântico
  `moved`), e o hover diz a quem ele foi entregue e, onde o nome ganha um valor novo, de
  onde vinha.

## 0.6.0

- "Uso a biblioteca de textos." com o realce de definição; os verbos das bibliotecas
  aparecem no completar, no hover e na assinatura como os do programa.

## 0.5.0

- Depurador (`lavrac dap`): F5 sem launch.json, pontos de parada, passos, pilha, variáveis
  com as partes, hover, parada nos erros de execução, e o terminal integrado para o
  programa que lê a entrada.

## 0.4.0

- Rename com morfologia: cada uso na forma em que foi escrito, o artigo e a contração no
  gênero do nome novo, a concordância que vai junto, e a recusa com o motivo quando o
  programa ficaria errado.
- "Corrigir tudo" (`source.fixAll`) com as correções seguras do compilador.

## 0.3.0

- Completar guiado pelo parser do compilador, com os verbos do programa como snippet da
  regência e os nomes do gênero e do número do artigo.
- Ajuda de assinatura com o parâmetro ativo, inlay hints de tipo e de modo de posse, ações
  rápidas com as correções do compilador, dobra e seleção crescente pelos blocos.
- Formatação do documento pelo `lavrac fmt`.

## 0.2.0

- Servidor de linguagem (`lavrac lsp`): diagnósticos ao digitar, com código e explicação;
  realce semântico; hover com tipo e posse; definição, referências, destaque e símbolos.
- Comandos de rodar, compilar e verificar, botão de rodar e tarefas com os erros no painel
  Problemas.
- Snippets dos blocos.
- A gramática tira as palavras do léxico do compilador.

## 0.1.0

- Realce de sintaxe e configuração da linguagem (comentários, pares, recuo, dobra).
