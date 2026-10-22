# Aplicações discutidas

O que sustenta todas: a Lavra junta três coisas que outras linguagens não juntam.

1. **Gramática como verificação de verdade**: concordância, regência, modo verbal,
   contração obrigatória, artigo definido (referência) e indefinido (criação), cada
   erro com o nome do fenômeno e a correção.
2. **Compilada, segura e rápida**: inteiros de 8 a 64 bits, byte, real de 32 e 64 bits,
   posse e empréstimo verificados, estouro e posição fora como falha com mensagem, e
   executável nativo.
3. **Verbos do usuário entram na língua**: quem define "Para transferir a quantia da
   origem para o destino" ganha "transfiro" com a regência na assinatura; predicados
   viram condição no subjuntivo; adjetivos flexionam. Uma biblioteca em Lavra cria
   vocabulário em português com regras gramaticais próprias.

## As oito aplicações

1. **Ensinar o que acontece embaixo do código**: posse, empréstimo, memória e
   representação de dados explicados na língua de quem aprende. Público: IFs, primeiro
   ano de computação e engenharia. Peça que falta: um modo passo a passo que mostre a
   frase executando e os valores mudando.
2. **Ensinar português com o compilador**: corretor gramatical sem exceção para um
   fragmento do português (regência, crase e contração, subjuntivo irregular,
   concordância nominal). `samples/tropeco.lav` é o formato do exercício. Conversa com
   o complemento de Computação da BNCC (2022). Peça que falta: banco de exercícios por
   fenômeno (os erros já saem classificados por categoria).
3. **Disciplina de compiladores**: todas as etapas clássicas, de léxico a código
   nativo, com verificações que são ao mesmo tempo compilação e linguística. Extensões
   possíveis: pretérito, mais backends.
4. **Microcontroladores e robótica educacional**: texto compilado em português entre o
   C++ do Arduino e os blocos. Caminho: backend para microcontroladores e uma
   biblioteca de verbos de hardware ("Ligo o pino 13", "Espero 500 milissegundos",
   "Leio a tensão do sensor"); pede cabeçalho com "artigo + substantivo + símbolo".
5. **Acessibilidade e programação por voz**: Lavra lida por leitor de tela soa como
   texto; dá para ditar sem comandos especiais; o compilador pode corrigir a
   transcrição ("o prova" para "a prova"). Serve a cegos, limitação motora, LER.
6. **Linguagem natural controlada para regras e especificação** (na linha do Attempto
   Controlled English, Inform 7, Catala): regras de cálculo legíveis por contador ou
   advogado. Faltam: tipo decimal para dinheiro, datas, tabelas de faixas como literal.
7. **Alvo para modelos de linguagem**: quem não programa consegue conferir o que veio;
   os erros (linha, fenômeno, correção) servem ao ciclo gerar, compilar e consertar; a
   redundância da gramática transforma erros de referência e de ordem de argumento em
   erro de compilação.
8. **Texto que roda**: concurso de programa-crônica, oficina com restrição no espírito
   do Oulipo (a restrição é compilar), divulgação. Ver [poems.md](poems.md).

## Ideias de poema pendentes

- Crivo de Eratóstenes: os números passam pela peneira e o que sobra são os primos.
