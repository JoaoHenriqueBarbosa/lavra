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

----------------------------------------------------------------------------------------

Dia 22/10/2026, fim das fases 2 e 3 do roadmap (formatador e ajuda à escrita), registro do Claude às 17h48, cerca de 2 horas e 25 minutos depois do começo:

O formatador (lavrac fmt) acerta o recuo, os espaços entre os tokens e o fim das linhas, e nunca muda quebra de linha, linha em branco ou o interior de um token, porque os poemas também são programas. O corpus inteiro já estava na forma; estragado de três jeitos (tabs, espaços sobrando, espaço no fim), volta byte a byte ao original, com os mesmos tokens e comentários, e formatar de novo não muda nada. O CI confere os samples e os programas certos com fmt --check, e o editor formata pelo servidor.

A ajuda à escrita sai do próprio compilador:
- O completar roda o parser com um marcador no lugar da palavra do cursor, e o parser diz o que aceitaria ali. O servidor só preenche: verbos da língua e do programa (o do programa entra com a regência para preencher), nomes do gênero e do número do artigo que você escreveu, campos, posições e tipos. Construção nova escrita com os auxiliares do parser já aparece no completar, sem mexer no servidor.
- Ajuda de assinatura com o parâmetro ativo, inlay hints de tipo e de modo de posse, ações rápidas com as correções do compilador, dobra e seleção crescente.

Conferido no VS Code de verdade, sob xvfb, pela API do editor e por três capturas. A captura do completar mostrou um espaço duplo depois do inlay hint de tipo, já corrigido. A integração do editor no CI falhava porque o cliente não era compilado antes do teste; passou a ser, e o CI ficou verde. Um commit da fase 2 entrou com um aviso do clippy porque encadeei as conferências com ";" em vez de "&&"; corrigi no commit seguinte e agora as conferências vão sempre encadeadas com "&&".

Decisões para revisar:
- O inlay hint de tipo usa a forma do aposto (", um inteiro,"), então o que o editor mostra é Lavra válida.
- Depois de um artigo, o completar só oferece nomes daquele gênero e número.
- A extensão foi a 0.3.0.

Próximo: a fase 4, o rename com morfologia e o lavrac fix.

----------------------------------------------------------------------------------------

Dia 22/10/2026, fim da fase 4 do roadmap (rename e correções), registro do Claude às 18h33, cerca de 3 horas e 10 minutos depois do começo:

O F2 do editor renomeia um nome em todos os lugares e em todas as formas, com a morfologia da língua. Uma função é reescrita em cada uso na forma em que foi escrita: "Rendo", "render", "exceder", "excederem", e o adjetivo do predicado concordando com cada sujeito. Um nome que muda de gênero leva junto o artigo e a contração ("do saldo" vira "da reserva"), com a maiúscula do começo da oração. Também vai junto o que concorda com ele sem citá-lo: "As compras começam vazias", "ambos" no cabeçalho, "é uma destas", "a primeira das notas". O "Se houver", os campos de um "Conforme" e o item de "Para cada" que deriva da coleção vão com o nome. Antes de devolver, o rename analisa o texto renomeado e recusa, dizendo o motivo, se aparecer erro ou se um uso passar a apontar para outro nome.

Conferido renomeando cada nome de cada programa do corpus, um de cada vez: 311 renomes, todos compilam e dão a mesma saída. Esse teste achou três casos que corrigi antes do commit:
- o adjetivo do cabeçalho tem de acompanhar o sujeito ("Para a peça ser quadrada");
- o item de "Para cada" tem o gênero da coleção, então trocar o gênero só dele agora é recusado com essa explicação;
- o item escrito que é o singular da coleção vai junto com ela ("Para cada herdeiro dos herdeiros").

O lavrac fix aplica as correções seguras em voltas, porque uma revela a outra: o ponto final que faltava escondia a concordância da linha de baixo. O editor ganhou o "corrigir tudo", que também roda ao salvar se você quiser. Ganharam correção a comparação no plural ("maiores que"), o adjetivo do predicado, "uma destas", o artigo do campo e a tipagem do cabeçalho. A correção que troca o começo de uma oração agora mantém a maiúscula; antes ela escrevia "a reserva" minúsculo no começo da frase.

A extensão foi a 0.4.0 e está instalada no seu VS Code no WSL, com o lavrac novo dentro. Recarregue a janela para ela entrar. A que estava lá era a 0.1.0: a 0.2.0 da fase 1 não tinha ficado instalada.

Decisões para revisar:
- Sem artigo, o rename só aceita o nome novo quando a terminação decide o gênero (-o, -a, -ção, -dade, -agem). "valor" e "problema" pedem o artigo.
- O número do nome não muda num rename, porque nome no plural é coleção. Símbolo troca só por símbolo.
- O ordinal acompanha a coleção quando tinha o gênero dela, embora a língua não confira essa concordância.
- Sem dicionário de substantivos, o singular derivado é o que a própria língua aceita: "os lápis" daria o item "lápi".

Próximo: a fase 5, o depurador (lavrac dap), sobre o interpretador.

----------------------------------------------------------------------------------------

Dia 22/10/2026, fim da fase 5 do roadmap (o depurador), registro do Claude às 19h03, cerca de 3 horas e 40 minutos depois do começo:

A Lavra tem depurador. O F5 no VS Code depura o arquivo aberto sem precisar de launch.json, e dá para ver a frase executando:
- pontos de parada por linha;
- próxima linha, entrar e sair de função, pausar e parar na entrada;
- a pilha com a linha de cada chamada ("Para render", "Para começar");
- os parâmetros, os locais e as constantes, só os que valem naquele ponto, com as listas abrindo em "o item 1", "o item 2" e os registros e casos abrindo nos campos;
- o valor no hover;
- a parada no erro de execução, onde ele aconteceu, com a pilha inteira.

O programa que lê a entrada roda no terminal integrado. O editor abre um lavrac debuggee, que liga de volta ao depurador por um socket local. Os outros programas rodam no console de depuração, com a entrada por arquivo se for preciso.

Por dentro, o interpretador passou a ser dono da pilha e a avisar um acompanhante antes de cada oração. O depurador é esse acompanhante e fica na biblioteca, sem depender do editor, então serve também ao playground da fase 9. Nenhuma saída mudou: golden, paridade com o nativo e o resto da suíte passaram iguais.

Conferido em três camadas:
- o núcleo, por canais;
- o binário, pelo protocolo, no console e no terminal, com o teste fazendo o papel do editor;
- o VS Code de verdade, parado num ponto, com a captura mostrando "o saldo = 100" e "as notas = [7; 8; 9]".

A primeira captura mostrava os locais fechados. Gravei o tráfego entre o VS Code e o depurador e comparei com o depurador de JavaScript nas mesmas condições: era o tempo do teste, não o depurador. O VS Code só abre o primeiro escopo sozinho se a vista de depuração já está à mostra quando o programa para.

A extensão foi a 0.5.0 e está instalada no seu VS Code. Recarregue a janela para ela entrar.

Decisões para revisar:
- Parar numa linha é chegar a ela vindo de outra: as voltas de um laço escrito numa linha só param uma vez.
- A falha declarada ("Desisto") só para com o filtro ligado; o erro de execução para sempre, com o filtro que vem ligado.
- Avaliar aceita só nomes, com ou sem artigo, e não expressões inteiras.

Próximo: a fase 6, as operações embutidas declaradas em sintaxe Lavra e a primeira biblioteca, "Uso a biblioteca de textos.".

----------------------------------------------------------------------------------------

Dia 22/10/2026, fim da fase 6 do roadmap (operações embutidas e bibliotecas), registro do Claude às 19h26, cerca de 4 horas depois do começo:

A Lavra tem bibliotecas. "Uso a biblioteca de textos." traz quatro verbos: "dividir o texto pelo separador", "juntar os textos com o separador", "o texto conter o trecho" ("Se a frase contiver ...") e "aparar o texto". A biblioteca é escrita em Lavra: cada operação é um cabeçalho comum que termina em ponto, sem corpo, e o próprio parser o lê. A regência, os tipos e a posse saem do cabeçalho. O interpretador e o nativo fazem a operação com a mesma saída, e o valgrind não acha vazamento.

O que isso garante para o futuro: uma operação nova toca quatro lugares:
- o cabeçalho na biblioteca;
- a linha na tabela de operações;
- a implementação no interpretador;
- a função no runtime.

Antes, uma operação embutida tocava nove. O servidor de linguagem atende a biblioteca sem nenhuma linha que saiba dela, e esse era o aceite da fase: completar com a regência, hover dizendo "Vem da biblioteca de textos.", assinatura e cores saem do vocabulário e das tabelas. A única regra nova do servidor é genérica: o que fica fora do documento não vira lugar nele.

Os testes acharam dois defeitos antes do commit:
- um diagnóstico com rótulo apontando para dentro da biblioteca derrubava o compilador ao ser mostrado; agora nada do programa aponta para lá;
- o rename confundia a posição da definição da biblioteca, que fica fora do texto e não se move com as edições.

Uma mudança na língua, registrada em "Mudanças": "uso" passou a ser palavra da língua, então o programa não define mais "Para usar ...". O teste do léxico exige que toda palavra que o parser consulta esteja nele, e "usar" virou um verbo da própria língua, narrado na 1ª pessoa como "Escrevo" e "Leio".

A extensão foi a 0.6.0, com o realce de "Uso a biblioteca de", e está instalada no seu VS Code. Recarregue a janela.

Decisões para revisar:
- "uso" reservado: "usar" deixa de estar disponível como verbo do programa.
- Dividir com o separador vazio dá cada caractere, em vez de falhar.
- Ir para a definição de um verbo de biblioteca não leva a lugar nenhum por enquanto. Mostrar o cabeçalho dela pede um documento virtual no editor, e fica para quando o site da fase 9 publicar as bibliotecas.

Próximo: a fase 7, as releases: binários, VSIX por plataforma, README, LICENSE e CHANGELOG na raiz.

----------------------------------------------------------------------------------------

Dia 22/10/2026, fim da fase 7 do roadmap (releases), registro do Claude às 20h11, cerca de 4 horas e 45 minutos depois do começo:

A Lavra tem a primeira release, a v0.1.0 no GitHub, com:
- o lavrac pronto para Linux, x86_64 (o PC comum) e aarch64 (ARM): um arquivo só, estático, que roda em qualquer Linux sem instalar nada;
- a extensão do VS Code de cada plataforma, com o lavrac dentro.

Conferido como um usuário faria, num Debian limpo e baixando da página da release. O checksum confere, e o lavrac roda um poema com a saída esperada. O de aarch64 também roda, pelo QEMU. A extensão traz o lavrac que roda, e com o gcc instalado o executável nativo dá a mesma saída. A cada release isso se repete sozinho no CI, com todos os poemas. Antes, provei que o lavrac estático compila para nativo tanto num sistema com glibc quanto num com musl, com o corpus inteiro.

Um defeito apareceu antes da release: o runtime embutido saía para a máquina que compila, e não para a máquina em que o lavrac roda. Um lavrac para ARM feito num PC levaria o runtime errado. Corrigido.

README, LICENSE e CHANGELOG estão na raiz, com a instalação explicada em linguagem simples. O release-plz abre o PR de cada versão, e só o seu merge publica; já há um aberto, o da 0.2.0, com o site.

Bloqueado, dependendo de você, porque são credenciais suas em serviços de fora:
- crates.io: o token de publicação. Não há token nesta máquina, e o navegador automatizado não está nesta sessão.
- Visual Studio Marketplace: o publisher joaohenriquebarbosa e o token (VSCE_PAT).
- Open VSX: a conta e o token (OVSX_PAT).

Com os tokens guardados como secrets do repositório, a pipeline publica sozinha. O crates.io pede ainda, uma vez, um cargo publish e a troca do release-plz para o modo do registro, e isso eu faço.

Decisões para revisar:
- A versão mínima do Rust é a 1.97, que é a do Cranelift.
- O release-plz tira as versões das tags enquanto a crate não está no crates.io, porque no modo do registro ele recusa a tag que o registro não tem.
- A primeira versão no crates.io vai ser a da última tag quando o token chegar.

----------------------------------------------------------------------------------------

Dia 22/10/2026, fim da fase 9 do roadmap (o site e o playground), registro do Claude às 20h12, cerca de 4 horas e 50 minutos depois do começo:

A Lavra tem site: https://joaohenriquebarbosa.github.io/lavra/. Ele sai inteiro do repositório, com:
- a apresentação;
- a referência da língua;
- o catálogo dos 185 códigos de erro, cada um com a explicação e os exemplos errado e certo;
- o léxico;
- uma página por poema, com a entrada e a saída.

Todo o código sai realçado pela mesma gramática da extensão do VS Code, no tema claro ou escuro do sistema. Ele se publica sozinho a cada envio para a main.

O playground roda o próprio compilador no navegador, compilado para wasm. Os erros aparecem no editor enquanto se escreve, o programa roda sem travar a página, e o botão de parar interrompe até um laço sem fim. O link de compartilhar leva o código junto. Conferido em três lugares:
- no Node, o wasm contra o corpus: 28 programas com a saída esperada;
- num Chromium de verdade: os 13 poemas rodam no playground com a saída esperada;
- no site publicado, a partir desta máquina: os mesmos 13. A captura mostra "A partilha", que lê entrada, rodando até o fim.

O workflow do site repete a conferência no Node e no navegador antes de publicar.

Um achado no caminho: no navegador, a pilha do motor JavaScript acaba por volta de 1 500 chamadas aninhadas da Lavra, bem antes das 10 000 do lavrac. O interpretador ganhou um limite configurável. No navegador ele vale 1 000, e quem passa dele recebe a falha da própria língua ("Estouro de pilha"), em vez de um erro do navegador.

Decisões para revisar:
- O playground limita a recursão a 1 000 chamadas aninhadas.
- O último programa escrito fica guardado no navegador de quem usa.

Falta a fase 8, a gramática tree-sitter com as configurações de Helix, Neovim e Zed. Um agente a está fazendo em paralelo, numa branch própria, e eu integro quando ele terminar.

----------------------------------------------------------------------------------------

Dia 22/10/2026, a parte adiada da fase 1 (a posse no editor), registro do Claude às 20h36, cerca de 5 horas e 15 minutos depois do começo:

O editor agora mostra a posse.
- O hover sobre o nome que entrega o valor diz a quem, e o que vale dali em diante: "As cartas são entregues a "arquivar" aqui: daqui em diante, não se usam mais."
- Onde o nome ganha um valor novo, o hover diz onde o antigo foi parar: "As cartas foram entregues a "arquivar" na linha 8; aqui ganham um valor novo."
- O nome que entrega o valor sai em itálico, pelo modificador semântico moved.

Tudo sai da própria análise de posse, que agora deixa essas notas nas tabelas. A extensão 0.7.0 está instalada no seu VS Code.

No caminho, as mensagens de posse passaram a concordar em número. Antes, um nome no plural saía "As cartas foi entregue ... e não volta". Agora sai "As cartas foram entregues ... e não voltam", "As saudações são constantes do programa e não se entregam" e "As notas já vão confiadas ... e não podem ir". Cinco programas errados no plural travam isso.

Conferido no servidor em processo e num VS Code de verdade, com a captura.

Decisões para revisar:
- O plano dizia que o valor entregue aparece apagado. O VS Code não dá opacidade a token semântico, e apagado é como ele já mostra o que não se usa. O nome sai em itálico, com a cor do tema.
- O itálico vem de uma regra padrão que a extensão contribui às customizações de token semântico, e que só liga o itálico. O caminho comum, mapear para o escopo markup.italic, trocava a cor do nome: os temas padrão pintam esse escopo (roxo no Dark+, verde no Abyss).
- O hover não repete o estado de posse nos usos depois da entrega, porque ali o erro já diz a mesma coisa.
