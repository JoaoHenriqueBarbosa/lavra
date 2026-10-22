# Aplicações discutidas

Da segunda conversa (2026-10-21). O que sustenta todas: a Lavra junta três coisas que
outras linguagens não juntam.

1. **Gramática como verificação de verdade**: concordância, regência, modo verbal,
   contração obrigatória, artigo definido (referência) vs indefinido (criação), cada
   erro com o nome do fenômeno e a correção.
2. **Baixo nível de verdade**: inteiros de 8 a 64 bits, byte, real 32/64, endereço,
   aritmética de ponteiro, reservar/liberar, registros com layout real, estouro que dá
   a volta, x86-64 nativo comentado com a frase original.
3. **Verbos do usuário entram na língua**: quem define "Para transferir a quantia da
   origem para o destino" ganha "transfiro" com a regência na assinatura; predicados
   viram condição no subjuntivo; adjetivos flexionam. Uma biblioteca em Lavra cria
   vocabulário em português com regras gramaticais próprias. É o ponto mais poderoso.

## As oito aplicações

1. **Ensinar o que acontece embaixo do código** (arquitetura/organização de
   computadores). A frase em português e o assembly lado a lado. Exercício testado:
   registrador de microcontrolador com união, interseção, inverso e deslocamento
   (137, depois 129, bits 7 e 0). Público: IFs, primeiro ano de CC/Engenharia.
   Falta: aba de memória mostrando as células mudando e modo passo a passo que
   destaca frase, instruções e células ao mesmo tempo (um "Python Tutor" de baixo
   nível em português).
2. **Ensinar português com o compilador**. Corretor gramatical sem exceção para um
   fragmento do português: regência, crase/contração, subjuntivo irregular (ver/vir,
   pôr/puser, ter/tiver), concordância nominal. O `tropeco.lav` é o formato do
   exercício. Conversa com o complemento de Computação da BNCC (2022). Falta: banco de
   exercícios por fenômeno (os erros já saem classificados em `ROTULO_ERRO`) e modo
   do editor que esconde x86 e destaca Morfologia.
3. **Disciplina de compiladores**. ~3.400 linhas com todas as etapas clássicas, cada
   uma numa aba. Extensões que são ao mesmo tempo compilação e linguística: pretérito
   ("guardei"), mais irregulares, backend RISC-V/ARM64, enumerações com concordância
   ("Uma cor é vermelha, verde ou azul").
4. **Microcontroladores e robótica educacional**. Texto compilado em português entre
   o C++ do Arduino e os blocos. Caminho: backend C, depois biblioteca de verbos de
   hardware ("Ligo o pino 13", "Espero 500 milissegundos", "Leio a tensão do
   sensor"). Bloqueio: cabeçalho com "artigo + substantivo + símbolo".
5. **Acessibilidade e programação por voz**. Lavra lida por leitor de tela soa como
   texto; dá para ditar sem comandos especiais; o compilador pode corrigir a
   transcrição ("o prova" → "a prova"). Serve a cegos, limitação motora, LER.
6. **Linguagem natural controlada para regras e especificação** (na linha do Attempto
   Controlled English, Inform 7, Catala). Regras de cálculo legíveis por contador ou
   advogado. Faltam: tipo decimal para dinheiro, datas, tabelas de faixas como literal.
7. **Alvo para modelos de linguagem**. Quem não programa consegue conferir o que veio;
   os erros (linha, fenômeno, correção) servem ao ciclo gerar/compilar/consertar; a
   redundância da gramática transforma erros de referência e ordem de argumento em
   erro de compilação.
8. **Texto que roda**. Concurso de programa-crônica, oficina com restrição no espírito
   do Oulipo (a restrição é compilar), divulgação. Ver [poems.md](poems.md).

## Roadmap sugerido na conversa

1. Aplicações 1 e 2 primeiro: aba de memória, modo passo a passo, banco de exercícios
   por tema. Mesmo editor servindo duas disciplinas da mesma escola.
2. Depois backend C com a extensão "o pino 13" no cabeçalho (robótica, qualquer
   arquitetura, WebAssembly no navegador).

Peças oferecidas para implementar e ainda não feitas: aba de memória, extensão do
cabeçalho para "o pino 13", primeiro gerador de C.

## Ideias de poema pendentes

- Crivo de Eratóstenes: os números passam pela peneira e o que sobra são os primos.
