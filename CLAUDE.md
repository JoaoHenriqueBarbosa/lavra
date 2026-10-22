# Lavra

Linguagem de programação de baixo nível e compilada em que cada instrução é uma
oração em português, e a gramática é verificação: artigo marca variável, modo verbal
marca o papel da oração, regência marca os argumentos, concordância é checagem
redundante. Extensão `.lav`.

## Estado

- `draft/` é o protótipo feito no claude.ai em 2026-10-21 (compilador JS,
  interpretador, gerador x86-64, runtime C, editor HTML, exemplos e três
  programas-poema). Funciona: o nativo bate com o interpretador nos exemplos.
- A Lavra vai ser **refeita** de acordo com as instruções do user. Trate o `draft/`
  como referência somente leitura: não edite, não gere artefatos dentro dele.
- Na raiz de `draft/`, `lavra.js` e `lavra.html` são mais novos que os de
  `draft/lavra/` (patch de contração no cabeçalho, necessário para
  `catilinaria.lav`).

## Notas

Leia antes de mexer em qualquer coisa: [`notes/README.md`](notes/README.md).

- `notes/origin.md`: o pedido, a tese de design, a ordem dos fatos.
- `notes/language.md`: a linguagem como está implementada (referência completa).
- `notes/draft-architecture.md`: como o draft é construído.
- `notes/draft-status.md`: o que foi verificado, bugs e limitações.
- `notes/applications.md`: as aplicações discutidas e o roadmap.
- `notes/poems.md`: os programas-poema e o que eles exigem da gramática.

Ao descobrir algo novo sobre a linguagem ou tomar uma decisão de design, atualize a
nota correspondente.

## Rodar o draft

Node está no host (`~/.local/bin/node`) e gcc também. `lavrac.js -s/-c` escreve `.s` e
executável ao lado do `.lav`, então rode numa cópia:

```bash
rm -rf /tmp/lavra-run && mkdir -p /tmp/lavra-run
cp -r draft/lavra/* /tmp/lavra-run/ && cp draft/lavra.js /tmp/lavra-run/ && cp draft/*.lav /tmp/lavra-run/exemplos/
cd /tmp/lavra-run
node lavrac.js exemplos/script.lav </dev/null      # interpreta (sempre dar stdin, ele lê até EOF)
node lavrac.js -c exemplos/lista2.lav && ./exemplos/lista2
```

O editor é `draft/lavra.html` (abre direto no navegador, sem servidor).

## Convenções para a reescrita

- **Identificadores de código em inglês** (classes, funções, variáveis, arquivos,
  pastas), mesmo que o draft use português (`Analisador`, `Maquina`, `falha`).
  A superfície da linguagem (palavras-chave, verbos-núcleo, tipos, mensagens de erro,
  exemplos `.lav`) é produto e continua em português.
- Prosa (comentários, docs, mensagens de erro, commits) em português com acentuação
  completa, sem travessão.
- Toda mudança na gramática precisa de programa certo e programa errado de teste, com
  a mensagem de erro esperada.
- Paridade entre interpretador e nativo é o critério de correção do backend: mesma
  saída byte a byte.
- Mudança visual no editor: screenshot pelo chrome-devtools antes de dar como pronto.
