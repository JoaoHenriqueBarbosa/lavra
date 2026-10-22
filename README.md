# Lavra

[![CI](https://github.com/JoaoHenriqueBarbosa/lavra/actions/workflows/ci.yml/badge.svg)](https://github.com/JoaoHenriqueBarbosa/lavra/actions/workflows/ci.yml)
[![Licença MIT](https://img.shields.io/badge/licen%C3%A7a-MIT-blue.svg)](LICENSE)

Uma linguagem de programação compilada em que cada instrução é uma oração em português, e a
gramática é a verificação: o artigo marca o nome, o modo do verbo marca o papel da oração, a
regência marca os argumentos, e a concordância é conferida pelo compilador.

```lavra
Para render o saldo, sendo o saldo um real confiado:
    O saldo cresce o saldo vezes 0,05.

Para começar:
    O saldo começa em 1000,0.
    Repito 2 vezes: rendo o saldo.
    Escrevo "rendido: " seguido do saldo.
```

```
rendido: 1102,5
```

É "um Rust com gramática e morfologia": posse e empréstimo, "talvez" e falha, registros e
enumerações, bibliotecas, e dois jeitos de rodar que dão a mesma saída, byte a byte: o
interpretador e o compilador para executável nativo. Os programas também podem ser poemas:
veja [`samples/`](samples).

**Experimente no navegador:** o [playground](https://joaohenriquebarbosa.github.io/lavra/playground.html)
roda o próprio compilador, com os erros ao digitar. O [site](https://joaohenriquebarbosa.github.io/lavra/)
tem a referência da língua, os poemas e a explicação de cada erro.

## Instalar

**1. O programa pronto (Linux).** Na página da
[última versão](https://github.com/JoaoHenriqueBarbosa/lavra/releases/latest), baixe o
arquivo do seu computador:

- `x86_64`: o PC comum, com processador Intel ou AMD;
- `aarch64`: os computadores ARM, como o Raspberry Pi e os servidores ARM.

É um arquivo só, que roda em qualquer Linux sem instalar mais nada. Descompacte e ponha o
`lavrac` numa pasta de programas:

```bash
tar xzf lavrac-*.tar.gz
sudo mv lavrac /usr/local/bin/
lavrac --version
```

Para transformar um programa em executável (`lavrac build`), o computador precisa também de
um compilador de C, o `gcc` ou o `clang`, que o `lavrac` usa para montar o executável no
final. Rodar, conferir e o editor funcionam sem ele.

**2. Pelo Cargo**, se você já tem o Rust (1.97 ou mais novo):

```bash
cargo install --git https://github.com/JoaoHenriqueBarbosa/lavra lavrac
```

**3. Do código-fonte:**

```bash
git clone https://github.com/JoaoHenriqueBarbosa/lavra
cd lavra
cargo build --release
./target/release/lavrac samples/caixa-de-poupanca.lav
```

## Usar

```
lavrac programa.lav                 roda o programa
lavrac build programa.lav           compila para um executável nativo
lavrac check programa.lav           só confere gramática, tipos e posse
lavrac fmt programa.lav             acerta recuo e espaços, sem mexer em quebra de linha
lavrac fix programa.lav             aplica as correções seguras (contração, concordância...)
lavrac explain L0334                explica um erro, com exemplo errado e certo
lavrac --help                       todos os comandos
```

Os erros dizem o que a gramática espera, com o código para a explicação:

```
Erro de concordância [L0113]: O verbo não concorda com o sujeito, que é plural: "forem", não "for".
 --> conta.lav:3:17
  |
3 |     Se as horas for maior que 10: escrevo "tarde".
  |                 ^^^
```

## No editor

A extensão do VS Code fica na página da versão (`lavra-vscode-*-linux-x64.vsix` ou
`-linux-arm64.vsix`) e já traz o `lavrac` dentro. Instale pela paleta, em "Extensions:
Install from VSIX...". Ela dá:

- os erros ao digitar, cada um com a explicação;
- as cores pelo sentido de cada palavra;
- o tipo e a posse no hover;
- o completar guiado pela gramática;
- o rename que acerta o artigo e a concordância;
- a formatação e as correções;
- o depurador (F5): pontos de parada, passos e variáveis.

O servidor de linguagem (`lavrac lsp`) e o adaptador de depuração (`lavrac dap`) servem a
qualquer editor que fale esses protocolos.

## Documentação

- [O site](https://joaohenriquebarbosa.github.io/lavra/): a referência, o catálogo de erros, o
  léxico e os poemas, gerados do repositório.
- [`notes/lavra.md`](notes/lavra.md): a referência da língua.
- `lavrac explain <código>`: cada erro explicado, com um exemplo errado e um certo.
- [`notes/compiler.md`](notes/compiler.md) e [`notes/tooling.md`](notes/tooling.md): o
  compilador, o nativo, o servidor de linguagem, o depurador e os testes.

## Licença

[MIT](LICENSE).
