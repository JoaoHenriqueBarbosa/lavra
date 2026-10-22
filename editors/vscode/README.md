# Lavra para VS Code

Suporte à [Lavra](https://github.com/JoaoHenriqueBarbosa/lavra), a linguagem compilada
em que cada instrução é uma oração em português.

## O que faz

- **Realce de sintaxe** (gramática TextMate `source.lavra`), com escopos padrão para
  qualquer tema: declaração (`é`, `começa em`), mutação (`vira`, `cresce`, `ganha`),
  controle (`Se`, `Enquanto`, `Repito`, `Para cada`, `Conforme`), cabeçalhos de função
  com parâmetros, particípios de posse (`emprestado`, `confiado`), tipos, registros e
  enumerações, operações por extenso, comparações, literais e comentários.
- **Edição**: comentário com `--`, pares e aspas, recuo depois de `:`, `Senão` volta o
  recuo, dobra por indentação.

O realce é léxico. O que depende de nomes declarados (o verbo de um predicado em
`Se o saldo exceder o teto`, um caso de enumeração) vem com o servidor de linguagem.

## Desenvolvimento

```bash
npm install
npm test                       # asserções de escopo (tests/syntax) e snapshots dos samples
npm run test:update-snapshots  # depois de revisar uma mudança intencional
npm run package                # gera lavra.vsix
code --install-extension lavra.vsix
```

A gramática é gerada por `syntaxes/build.py` (as listas de palavras ficam lá); não edite
o JSON à mão. Os testes de escopo são gerados por `scripts/make_syntax_tests.py`.
`node scripts/tokens.mjs arquivo.lav` mostra os escopos de cada token, e
`node scripts/preview.mjs saída.html arquivos.lav...` renderiza nos temas Dark+ e Light+.
