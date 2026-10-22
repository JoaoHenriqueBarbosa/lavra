Dia 22/10/2026, implementação da Lavra em Rust, mensagem final do Claude, depois de aproximadamente 60 minutos de trabalho contínuo:

A Lavra está implementada do início ao fim do plano, da F0 à F4. Programas certos e errados rodam no interpretador e viram executáveis nativos pelo Cranelift, com saída idêntica nos dois.

O que funciona
- A língua inteira do plano:
  - declarações com "é" (fixo) e "começa em" (variável), mutação com o dado como sujeito ("vira", "cresce", "ganha", "perde", "Tomo");
  - funções com regência, predicados no subjuntivo, o fato de e conversão com como;
  - registros, listas, enumerações com Conforme exaustivo;
  - talvez/Se houver, e falha com salvo uma falha, Desisto com e , ou desisto.
- O verificador de posse responde em português: "A carta foi entregue a "arquivar" na linha 6 e não volta", com a dica de pedir emprestado ou entregar uma cópia. Ele segue o programa pelos caminhos de Se e pelas voltas dos laços, e acusa o duplo confiado na mesma chamada e a coleção que muda dentro do próprio Para cada.
- Desempenho: o nativo foi 82 vezes mais rápido que o interpretador (0,35 s contra 28,7 s para contar os primos até 2 milhões), com checagem de estouro em toda operação.
- "Lavra dos dias" escrito em Lavra, como teste de regressão: ganhou "uma data com o dia o sol, o mês a lua e o ano o tempo".

Verificação: 76 programas na suíte (10 certos, 66 errados com a mensagem exata) e 2 exemplos. A paridade entre interpretador e nativo compara stdout, stderr e código de saída. O valgrind, num contêiner, não achou vazamento nem acesso inválido nos 18 executáveis. Clippy passa com -D warnings.

Morfologia: os cinco agentes conferiram as tabelas contra o dicionário inteiro:

┌────────────┬──────────────────┬──────────────────────────────────────┐
│   Tabela   │    Conferido     │             Divergências             │
├────────────┼──────────────────┼──────────────────────────────────────┤
│ 1ª pessoa  │ 15 267 verbos    │ 0                                    │
├────────────┼──────────────────┼──────────────────────────────────────┤
│ subjuntivo │ 11 716 verbos    │ 0                                    │
├────────────┼──────────────────┼──────────────────────────────────────┤
│ 3ª pessoa  │ 11 715 verbos    │ 0                                    │
├────────────┼──────────────────┼──────────────────────────────────────┤
│ plural     │ 286 391 palavras │ 8, todas erros do próprio dicionário │
├────────────┼──────────────────┼──────────────────────────────────────┤
│ gênero     │ 136 571 pares    │ só as documentadas                   │
└────────────┴──────────────────┴──────────────────────────────────────┘

Decidi os casos de borda que eles trouxeram; as pendências estão em notes/morphology.md.

Decisões minhas que vale você revisar:
- Posições começam em 1 também em o item 1.
- talvez é tipo dono e se move, mesmo de inteiro (no nativo é um ponteiro que pode ser nulo).
- Nome no plural é sempre coleção.
- Lista sai como [7; 8,5; 9].
- Predicado com "ir" é proibido (o subjuntivo "for" colide com o de "ser").
- Verbos cuja 1ª pessoa é palavra da língua não podem ser definidos (comer → "como").

Limites: ainda não há referências guardadas em variáveis, tempos de vida, genéricos nem traits; ficam para depois, como o plano já dizia. Registros vivem no monte, o que é correto mas não é custo zero.

----------------------------------------------------------------------------------------

Depois pedi, ainda nesse dia, que o Claude escrevesse em Lavra os outros programas e poemas. Resposta dele depois de mais ou menos 10 minutos:

Os samples/ agora têm os três poemas e sete exemplos. Todos rodam e entram na suíte golden e na de paridade; a suíte, a paridade e o valgrind (28 executáveis, nenhum problema de memória) passam, e o clippy está limpo.

┌─────────────────────────────────┬───────────────────────────────────────────────┬────────────────────────────────┐
│             Sample              │                    O que é                    │          Conferência           │
├─────────────────────────────────┼───────────────────────────────────────────────┼────────────────────────────────┤
│ catilinaria.lav                 │ Primeira Catilinária                          │ os dois desfechos conferidos   │
│                                 │                                               │                                │
├─────────────────────────────────┼───────────────────────────────────────────────┼────────────────────────────────┤
│ a-partilha.lav                  │ A partilha                                    │ conferida (13 por 21 braças,   │
│                                 │                                               │ três herdeiros)                │
├─────────────────────────────────┼───────────────────────────────────────────────┼────────────────────────────────┤
│ ab-urbe-condita.lav             │ reconstruído a partir do cabeçalho, da        │ reproduz os 690 anos, 6 meses  │
│                                 │ chamada e da saída registrados na conversa    │ e 17 dias, ano 691             │
├─────────────────────────────────┼───────────────────────────────────────────────┼────────────────────────────────┤
│ lavra-dos-dias.lav              │ (já estava)                                   │ conferido                      │
├─────────────────────────────────┼───────────────────────────────────────────────┼────────────────────────────────┤
│ script, media, regencia,        │ exemplos                                      │ rodam e entram na suíte        │
│ sequencias, completo            │                                               │                                │
├─────────────────────────────────┼───────────────────────────────────────────────┼────────────────────────────────┤
│ pilha.lav                       │ pilha de números numa lista                   │ 385                            │
│                                 │                                               │                                │
├─────────────────────────────────┼───────────────────────────────────────────────┼────────────────────────────────┤
│ tropeco.lav                     │ erro de concordância de propósito             │ a expectativa é a mensagem de  │
│                                 │                                               │ erro                           │
└─────────────────────────────────┴───────────────────────────────────────────────┴────────────────────────────────┘

O que a língua pediu dos versos. A Lavra não tem variável global que muda, então o estado viaja por parâmetros confiados, e isso entrou no texto:
- Na Catilinária, os oradores passam pela regência, em espelho: "Acuso o réu pelo cônsul com …" e "Respondo ao cônsul pelo réu com …"; e o senado é consultado "sobre o cônsul e o réu".
- Na Partilha, "Corto os quadrados da sobra no dia" e "Giro a sobra" ("virar" é verbo da língua).
- Os herdeiros são uma lista que se libera sozinha.

Está tudo registrado em notes/poems.md.

Quatro bugs do compilador que esses programas revelaram, já corrigidos:
- Ambiguidade silenciosa do seguido de. O último argumento de uma chamada engolia o resto da frase, e como o resultado era um texto válido, nenhum erro aparecia. Agora o seguido de fica fora dos argumentos; para juntar texto dentro de um argumento, parênteses. Ganhou teste próprio.
- Leio o nome da viúva tirava o texto de dentro do registro ao guardar "o resultado". Agora o resultado é uma cópia.
- ambas contava o complemento fixo como parâmetro. Agora se refere aos dois regidos.
- Campo de registro com símbolo (x, um real;) e a mistura de números escritos (a potência de 2 e 0,5) passaram a funcionar.

A leitura ambígua do Ab urbe condita, que dá 689 em vez de 691, é erro de compilação, e virou teste.

Ficaram de fora do completo as conversões de texto para número e de caractere para inteiro, que a Lavra ainda não tem.

----------------------------------------------------------------------------------------

Dia 22/10/2026, plano de tooling e de crescimento da língua (notes/roadmap.md), metade da fase 0, registro do Claude depois de aproximadamente 60 minutos de trabalho:

Feito e enviado, com a suíte verde em cada passo:
- CI no GitHub Actions: formato, clippy, testes, extensão e valgrind em cada push (o repositório virou público para isso).
- Robustez: cerca de 16 mil versões mutiladas dos programas do corpus passam pela análise sem pânico, sem travar e sem apontar para fora do texto. A primeira rodada achou um laço infinito (parêntese sem par), um unwrap em "Para cada i:" sem coleção e um sublinhado que partia uma letra acentuada; os três viraram testes.
- Léxico como fonte única (src/lexicon.rs): toda palavra da língua com papel e documentação. O parser só consulta palavras do léxico (um teste confere), as formas reservadas saem dele, e a gramática do VS Code e o conferidor da morfologia leem a exportação (editors/lexicon.json). Contra os 15 267 verbos do Verbiste, só emprestadar e vaziar passaram a ser proibidos (registrado em Mudanças).
- Vocabulário do programa numa struct própria, base das bibliotecas e do completar.
- Códigos estáveis em todos os diagnósticos ("Erro de escopo [L0334]: ..."), inclusive nos erros de execução dos dois back-ends.

Em andamento: a análise que continua depois do primeiro erro (léxico e parser prontos, falta a semântica e a posse).

Decisões para revisar:
- O código aparece no cabeçalho do erro, entre colchetes.
- As formas recusadas com dica ("guardo", "seja", "há") não ficam reservadas: o usuário pode definir "guardar".
- O artigo antes de "o tamanho", "o resultado" e "o item" ganhou a cor da palavra que ele determina.

Um travamento do fluxo de resposta interrompeu o trabalho uma vez; para não depender de alguém acordar a sessão, há agora um despertador a cada 10 minutos que só dispara com a sessão parada, e o estado exato da execução fica gravado na memória do projeto.

----------------------------------------------------------------------------------------

Dia 22/10/2026, fim da fase 0 do roadmap (as fundações), registro do Claude depois de aproximadamente 1 hora e 40 minutos de trabalho:

A fase 0 terminou, com a suíte verde e o CI verde em cada envio. Desde o registro anterior:
- A análise não para mais no primeiro erro. O léxico, o parser, a semântica e a posse relatam e seguem, sem cascata: nenhum dos 72 programas errados mudou de saída, e seis programas novos com vários erros mostram cada um uma vez.
- Correções automáticas onde o compilador sabe a forma certa (contração, concordância, subjuntivo, ponto final, vírgula de "Senão, se"), para o editor e para o lavrac fix.
- lavrac explain: 183 explicações, uma por código, com exemplo errado e certo, conferidas por teste. Cinco agentes escreveram em paralelo, e escrever os exemplos revelou defeitos que corrigi: o nativo quebrava em "o módulo de" inteiro de 64 bits; lista de enumeração não era tipo; registro e enumeração podiam ter o mesmo nome; a sobrecarga sem preposição saía errada; e mensagens sem contração ou concordância.
- lavrac check --json, HIR com o trecho de cada oração, posições em UTF-16 para os editores e o mapa de arquivos.
- Tabelas de símbolos: cada nome declarado e usado, os escopos e o tipo de cada expressão, que o servidor de linguagem vai consultar. Daí vieram os avisos de função nunca chamada e de nome nunca usado; os samples não têm nenhum.
- Linha de comando própria e em português, com --version e o commit; o back-end nativo virou opcional, e a análise com o interpretador compila para wasm32 (o playground).

Decisões para revisar:
- Os avisos de não uso só aparecem em programa sem erro nenhum.
- Fiquei com uma linha de comando própria em vez do clap, porque o clap mostraria a ajuda e os erros de uso em inglês.
- Títulos de dois códigos mudaram para dizer o que o compilador faz: L0401 é mudar enquanto emprestado, L0405 é entregar enquanto emprestado.

Pendências de mensagem que os agentes anotaram e ficaram para depois: "sendo p e q um real" dá um erro genérico em vez do L0165; "Uns dados são um registro de:" não é reconhecido como definição.

Próximo: a fase 1, o servidor de linguagem (lavrac lsp) e o cliente do VS Code.

----------------------------------------------------------------------------------------

Dia 22/10/2026, fim da fase 1 do roadmap (o servidor de linguagem), registro do Claude às 17h17, cerca de 2 horas depois do começo:

O VS Code agora fala com o próprio compilador. lavrac lsp mostra todos os erros ao digitar, com o código e o link para a explicação; pinta os verbos do programa (também nos predicados), os parâmetros emprestados e confiados, o que é fixo e o que muda, campos, tipos e casos; mostra no hover o tipo por extenso, o modo de posse e a regência de cada verbo; e leva à definição, às referências e aos símbolos. A extensão ganhou os comandos de rodar, compilar e verificar, o botão de rodar, tarefas com os erros no painel Problemas e snippets dos blocos; a versão 0.2.0 está instalada no seu VS Code, com o lavrac dentro.

Conferido de duas formas: o servidor em processo, por uma conexão em memória, e a extensão num VS Code de verdade, sob xvfb num contêiner, que abriu um programa com erro e conferiu diagnósticos, hover, tokens e definição pela API do editor (o CI roda o mesmo teste). A captura mostra o hover "o saldo: variável, um inteiro. Declarado na linha 5." e o erro sublinhado.

Ainda não estão no hover o estado de posse naquele ponto ("entregue a arquivar na linha 6") e o modificador de valor já entregue: a análise de posse ainda não guarda o estado por uso. Fica para quando o depurador mexer no interpretador e nas tabelas.

Próximo: a fase 2, o formatador (lavrac fmt), que não mexe em quebra de linha nenhuma, por causa dos poemas.
