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

----------------------------------------------------------------------------------------

Dia 22/10/2026, duas pendências anotadas pelo caminho, registro do Claude às 20h48, cerca de 5 horas e 35 minutos depois do começo:

As pendências de mensagem que os agentes tinham anotado estão resolvidas:
- "sendo p e q um real" dava um erro genérico de sintaxe no "e". Agora dá o L0165: vários parâmetros pedem o tipo no plural. A dica oferece dar a cada parâmetro o seu, e vale também para "sendo ambos um real".
- "Uns dados são um registro de:" e "Umas formas são uma destas:" não eram reconhecidos como definição. A linha virava oração, com uma cascata de erros nos campos. Agora dá só o L0201: o tipo se define no singular.

A definição de um verbo de biblioteca, que eu tinha deixado para quando o site publicasse as bibliotecas, agora leva ao texto da biblioteca. No VS Code, ele abre só para leitura, no cabeçalho do verbo. O servidor só manda esse endereço ao editor que avisa que sabe abri-lo; nos outros, o hover continua dizendo de que biblioteca o verbo vem. O site ganhou a página das bibliotecas.

Um defeito antigo do site apareceu na captura do tema escuro: cada trecho realçado pintava o próprio fundo com a cor do tema do VS Code, e isso deixava retângulos sobre o fundo dos blocos de código. Corrigido em todas as páginas.

Decisão para revisar: o endereço da biblioteca no editor é "lavra-library:/textos.lav", e a aba mostra "textos.lav".

----------------------------------------------------------------------------------------

Dia 22/10/2026, uma auditoria do roteiro contra o que existe, registro do Claude às 20h59, cerca de 5 horas e 50 minutos depois do começo:

Conferi o roteiro inteiro, item por item, contra o código, e achei duas coisas que o plano pedia e que tinham ficado de fora de fases já marcadas como feitas:
- As configurações que desligam cada tipo de inlay hint (lavra.inlayHints.types, .modes e .result). Agora existem, e o servidor as lê de um jeito que serve a qualquer editor: nas opções de inicialização, que é por onde o Helix manda a configuração, e na mudança de configuração, que é como o VS Code e o Neovim mandam. Quando a escolha muda, o servidor pede ao editor que busque as dicas de novo. Conferido no VS Code de verdade, desligando e religando.
- Os roteiros dos pontos de extensão que ainda não têm uso real (módulos, genéricos, referências guardadas, tipos novos no runtime) e a lista do que uma feature nova toca. Estão em notes/compiler.md, na seção "Crescer a língua", cada um partindo do que o código já tem.

Decisão para revisar: a extensão procura o lavrac no setting, depois o que vem dentro dela, e só então no PATH. O plano dizia PATH antes do embutido. Com o embutido primeiro, o servidor é sempre o da mesma versão da extensão, e um lavrac antigo no PATH não tira recursos dela. Quem quiser outro diz o caminho no setting.

----------------------------------------------------------------------------------------

Dia 22/10/2026, fim da fase 8 do roadmap (o tree-sitter e os outros editores) e do roteiro inteiro, registro do Claude às 21h56, cerca de 6 horas e 40 minutos depois do começo:

A Lavra agora tem gramática tree-sitter e funciona no Helix, no Neovim e no Zed, além do VS Code.
- A gramática é rasa de propósito: orações, blocos, cabeçalhos, definições, comentários, textos, números, as palavras da língua pelo papel e os nomes com artigo. O detalhe fino continua com o servidor de linguagem.
- As palavras vêm do mesmo léxico do compilador, e um scanner em C cuida do recuo e do fim da oração, pelas regras do léxico do compilador.
- As queries de realce, dobra, recuo, escopos e objetos de texto são escritas uma vez, e as cópias de cada editor saem geradas.
- Helix, Neovim e Zed têm cada um a sua configuração, com README de instalação. No Neovim, até o texto das bibliotecas abre na definição, como no VS Code.

Conferido por um agente, que fez a fase numa branch própria, e de novo por mim antes de integrar:
- os 47 testes de corpus e as 67 asserções de realce passam, e as 20 queries compilam contra a gramática;
- a deriva: os 28 programas do corpus, os 10 da gramática e as 4 fixtures do VS Code parseiam sem nó de erro, e os 91 programas errados não derrubam o parser;
- no Helix 25.07.1, o hx --health lavra dá ✓ no servidor, no depurador, no parser e nas três queries, e um programa aberto, com recuo e objeto de texto usados, não deixa erro no log;
- no Neovim 0.11 e 0.12, as queries carregam, os programas abrem sem erro, o servidor publica os diagnósticos e o moved sai em itálico;
- a extensão do Zed compila.

O CI ganhou um job que roda tudo isso, menos o Neovim e o Zed, a cada envio, e passou inteiro.

Ficou de fora, por limite dos próprios editores: o depurador no Zed, que pede um adaptador escrito na extensão; os tokens semânticos no Helix, que não os pede; e o texto das bibliotecas no Helix e no Zed, que só abrem arquivos.

Decisões para revisar:
- As configurações do Helix e do Zed apontam a gramática por um commit (o rev), e o CI confere que a gramática naquele commit é a de agora. Toda mudança na gramática vai no mesmo envio que a troca do rev, e a integração tem de manter o hash: avanço rápido ou merge, nunca rebase.
- As queries são escritas com os nomes de captura do Neovim, e as dos outros editores saem trocando só os nomes. Por isso o Helix precisa ser 25.07 ou mais novo.
- O parser sai com a ABI 14, e o tree-sitter-cli fica preso na versão 0.27.1.

Com isso o roteiro inteiro está feito: as fases 0 a 9, as partes adiadas e as lacunas da auditoria. Ficam só coisas que dependem de você:
- os tokens de publicação: o do crates.io, para o primeiro cargo publish da última tag, e os do Marketplace e do Open VSX, para a extensão. Só quem tem a conta gera, e a pipeline já os usa quando chegarem;
- o merge do PR de release que o release-plz abriu (v0.2.0), que é seu.

Dia 10/11/2026, começo do catálogo de features do roadmap (seção 2.4), registro do Claude:

O pedido é implementar o catálogo inteiro, menos os tokens de publicação e o nativo em macOS e Windows. Cada item sai num commit e num envio só, na main, com a entrada aqui e no CHANGELOG, que ganhou a seção "Unreleased" (o release-plz a reconhece e põe a versão nova abaixo dela).

Antes do catálogo, duas mudanças pedidas na conversa, já enviadas:
- "salvo engano" no lugar da locução antiga do cabeçalho (c4a3bda). É incompatível: "engano" virou palavra da língua.
- o tipo implícito, "sendo a prova emprestada" (25391b0), com o L0176 e o rename que escreve o tipo por extenso.

Item 1, "talvez" no cabeçalho:
- "sendo o apelido talvez um texto emprestado" e "devolvendo talvez um inteiro". A sema e os dois back-ends já tratavam o "talvez" como tipo qualquer; faltava só o parser do cabeçalho aceitar.
- Quem chama passa o valor, "nada" ou um nome que já é "talvez". "talvez talvez" dá o L0126 de sempre, e o plural ("sendo a e b talvez inteiros") é recusado: cada parâmetro que pode faltar tem a sua tipagem.
- Um programa certo (com a paridade) e dois errados.

Item 2, conversões com "como":
- Texto para número: "42" como inteiro, "3,5" como real, em qualquer largura. Usa o mesmo leitor do "Leio um inteiro", então a vírgula decimal e os espaços das pontas valem igual.
- Caractere para inteiro (o código do caractere) e inteiro para caractere, que não estava no catálogo mas é a volta natural.
- Dois códigos de execução novos: L0915 (o texto não é número) e L0916 (o número não é código de caractere). O caractere que não cabe no inteiro pedido ("€" como byte) usa o L0908 de sempre.

Decisão para revisar: texto que não é número interrompe o programa, como a entrada inválida do "Leio". Não há forma de tentar e seguir (um "como inteiro, ou 0"); se fizer falta, cabe como verbo da biblioteca de textos devolvendo "talvez".


Item 3, as formas que estavam só na AST (`CondKind::Empty`, `CondKind::Contains`, `Pattern::Literal`):
- "Se as compras forem vazias", "Se o nome for vazio": vira tamanho igual a zero na sema, sem nada novo nos back-ends. Um predicado "ser vazio" definido pelo programa continua valendo antes (o corpus tem dois).
- "Se houver 7 nas notas", "Se não houver "ovo" nas compras", "Se houver "avr" no nome": uma condição nova na HIR, com três funções no runtime. A igualdade é a mesma do "for igual a".
- "Conforme o dia: Sendo 1 ou 7: ... Senão: ...": baixa para um "Se" encadeado, com o valor lido uma vez (num local escondido quando não é um nome). Vale para inteiro, caractere, texto e lógico.
- Quatro códigos novos (L0363 a L0366); o L0309 mudou de sentido (agora acusa real, lista e registro no "Conforme") e o L0332 continua reservado.

Decisões para revisar:
- A pertinência se diz com "houver ... em", e não com "conter": "conter" é verbo da biblioteca de textos, e um verbo da língua com o mesmo nome colidiria com ela. O "houver" já existia, e a forma não reserva palavra nova.
- "Conforme" sobre real é recusado, porque igualdade exata de real quase nunca é o que se quer.

Item 4, numerais por extenso:
- De "zero" a 999 999, no léxico: os simples e os compostos como a língua os escreve ("vinte e cinco", "cento e vinte e três", "dois mil e trezentos", "mil duzentos e trinta"). O lexer junta o composto num número só, na mesma linha.
- Os que flexionam concordam: feminino antes de "vezes" ("duas vezes", "vinte e uma vezes", "duzentas vezes"), masculino no resto. A discordância é o L0013, com correção.
- As gramáticas dos editores pintam as palavras como número.

Decisões para revisar:
- "vinte e cinco" é sempre 25, nunca a lista de 20 e 5 (para a lista, algarismos). O mesmo vale dentro de "a soma de vinte e cinco": ali é um número só, e a soma fica sem o segundo termo.
- "um" e "uma" sozinhos continuam artigos, então 1 não se escreve por extenso fora de um composto ("Para cada i de 1 até três").
- As palavras dos numerais ficam reservadas (mudança incompatível, registrada em "Mudanças" de notes/lavra.md). Sem milhões.

Um conserto no meio do caminho: os numerais reservaram "zero", e "zerar" deixou de poder ser verbo do programa (a 1ª pessoa é "zero"). Como o número nunca abre oração e o verbo sempre abre, não há ambiguidade: o verbo com forma de numeral voltou a valer ("Zero o contador."). O CI também estava vermelho desde o "salvo engano", por uma regra só: o rev da gramática no Helix e no Zed tem de apontar para o commit que mudou a gramática gerada. Cada envio que mexe no léxico agora leva um segundo commit com o rev.

Item 5, interpretador sem cópia nas chamadas:
- O parâmetro confiado, e o emprestado com dono quando o argumento é um lugar, chegam como endereço na pilha (o quadro, o nome e o caminho). A função lê e muda no lugar de quem chamou, e o que ela repassa a outra função leva o endereço original. A cópia na ida e na volta saiu.
- Medido: 3 000 chamadas com uma lista de 200 000 itens (uma emprestada e uma confiada por volta) caíram de 12,4 s para 0,01 s. A saída é a mesma, e a paridade com o nativo confere o corpus inteiro.
- O depurador mostra o parâmetro pelo valor que está no quadro de quem chamou.
- Ainda copia: o emprestado que é um temporário, e o "Para cada", que lê a coleção uma vez por laço.

Item 6, registro recursivo por lista:
- "as subpastas, uma lista de pastas" dentro de uma pasta, "uma adição, com as parcelas, uma lista de expressões" dentro de uma expressão, e os ciclos entre dois tipos. Árvores e expressões aninhadas já se escrevem.
- O tipo passa a existir antes dos campos, e o campo guarda o próprio tipo. Os back-ends não mudaram: o descritor do nativo já se registrava antes de descer, e o interpretador anda pelo valor.
- Conter a si mesmo sem lista continua sendo o L0302, agora com a dica da lista. O L0303 ficou reservado.

Decisões para revisar:
- "talvez" continua não quebrando o ciclo (a explicação do L0302 já dizia isso). No nativo ele é um ponteiro e caberia; se você quiser lista encadeada com "o próximo, talvez um nó", é uma mudança pequena.
- O tipo recursivo é um ciclo de Arc, que não é liberado: alguns bytes por análise. No servidor de linguagem isso se acumula numa sessão longa com tipo recursivo aberto. Está anotado em notes/compiler.md.

Item 7, bibliotecas, primeira parte (textos):
- Seis verbos novos: "maiusculizar o texto", "minusculizar o texto", "inverter o texto", "substituir o trecho pelo novo no texto", "localizar o trecho no texto" e "recortar o texto desde o início até o fim".
- "localizar" é a primeira operação que devolve "talvez" (a posição, ou nada), e "recortar" a primeira que recebe números; as posições são de caractere e começam em 1, como no resto da língua.
- O recorte fora do texto é falha de execução (L0917). A lógica de cada operação mora no arquivo que o interpretador e o runtime compartilham.

Decisão para revisar: os verbos "maiusculizar" e "minusculizar" existem no dicionário, mas são pouco usados. Preferi um verbo próprio a uma locução como "pôr as maiúsculas no texto".

Item 7, segunda parte (matemática):
- "Uso a biblioteca de matemática." traz seno, cosseno, tangente, logaritmo, exponencial, piso, teto e pi como operações por extenso ("o seno de x", "o pi"), e os verbos "arredondar x", "sortear desde o mínimo até o máximo" e "semear a semente".
- A operação por extenso de biblioteca é nova: a biblioteca a declara com "Para calcular o seno de x, ...", e ela se usa pelo nome, como "a raiz de x" da língua. Não reserva nada em quem não traz a biblioteca, e o verbo "calcular" continua livre para o programa.
- O sorteio é o mesmo algoritmo no interpretador e no nativo: com a mesma semente, a mesma sequência (é o que deixa testar pela paridade). Sem semente, começa pelo relógio.
- Logaritmo de zero ou de negativo é falha de execução (L0918), como a raiz de negativo.

Decisões para revisar:
- As funções vieram como nomes, e não como verbos: português não tem verbo para "seno", e um verbo genérico como "calcular" tomaria do programa um verbo que todo mundo usa. O custo: no programa que traz a biblioteca, "o pi" é a constante, e não pode ser nome de variável.
- Ângulos em radianos.

Item 7, terceira parte (arquivos):
- "Uso a biblioteca de arquivos." traz "carregar o arquivo", "gravar o texto no arquivo", "acrescentar o texto ao arquivo", "apagar o arquivo", "listar a pasta" e a condição "Se o caminho existir". O arquivo é o caminho, um texto.
- São as primeiras operações da língua que podem falhar: o cabeçalho leva "salvo engano", e a chamada tem de dizer o que fazer, como com uma função do programa. A falha explica em português ("Não consegui carregar "notas.txt": não existe."), sem o texto do sistema, que mudaria de máquina para máquina.
- No nativo, a operação recebe o endereço de uma casa onde deixa a falha.

Decisão para revisar: o arquivo inteiro é lido e gravado de uma vez, como texto. Não há leitura linha a linha nem arquivo aberto: para as linhas, "dividir" o conteúdo por "\n", da biblioteca de textos.

Item 7, quarta parte (tempo), que fecha as bibliotecas:
- "Uso a biblioteca de tempo." traz "o agora" (o instante, em milissegundos desde 1970), as partes de um instante ("o ano de x", "o mês de x", "o dia de x", "a hora de x", "o minuto de x"), "datar o instante" (o texto "10/11/2026 14:57:03") e "aguardar o tempo".
- As partes saem no horário do lugar, pedido ao sistema; no navegador (o playground), saem no horário universal, e "o agora" vale zero.
- Os testes só conferem relações (o ano é pelo menos 2026, a espera durou o que pediu), porque a saída tem de ser igual em qualquer máquina e em qualquer fuso.

Decisões para revisar:
- O instante é um inteiro, e não um registro "data": uma biblioteca ainda não define tipos, só operações. Um tipo "data" com os campos caberia quando houver módulos.
- Não há "o segundo de x": "o segundo de" já é a posição 2 de uma coleção, e as duas leituras brigariam no programa que traz a biblioteca. O segundo sai da conta, "o resto de (x dividido por 1 000) e 60".
- "o agora" é um nome, como "o pi". No programa que traz a biblioteca de tempo, não serve de nome de variável.

Item 8, conjuntos e mapas:
- Dois tipos novos, com a sintaxe da lista: "um conjunto de textos" e "um mapa de textos para inteiros".
- O conjunto ganha e perde o próprio valor ("Os vistos ganham "Ana".", "Os vistos perdem "Ana"."), responde a "Se houver "Ana" nos vistos" e tem união, interseção e diferença, com os nomes que a língua já tinha ("a união de A e B").
- O mapa usa "o item" com a chave no lugar da posição: "O item "Ana" das idades vira 30." põe ou troca, "o item "Ana" das idades" lê (a chave que falta é falha de execução, L0919), "As idades perdem "Ana"." tira, e "Para cada nome das idades" passa pelas chaves.
- Os dois ficam em ordem (dos valores, ou das chaves), então a escrita e o laço saem sempre iguais, e a paridade com o nativo confere. Escrevem-se "{Ana; Bia}" e "{Ana: 30; Bia: 25}".
- Três códigos novos: L0367 (chave de tipo sem ordem), L0368 (operação que a coleção não tem) e L0919.

Decisões para revisar:
- Chave e valor de conjunto só de inteiro, caractere, texto ou lógico. Real ficou de fora pelo arredondamento; registro, por não ter ordem.
- Não há valor de reserva na leitura de uma chave ("o item "Ana" das idades, ou 0"): pergunta-se antes com "Se houver". Se fizer falta, é uma forma nova pequena.
- O mapa começa sempre vazio; não há jeito de escrever um mapa com os pares de uma vez.
- "Para cada" num mapa dá as chaves, e o valor se lê pelo item. Não há laço que dê os dois.
- O valgrind sobre a suíte inteira (num contêiner) passou nos conjuntos e mapas e achou um vazamento antigo, que o teste da biblioteca de arquivos tocou pela primeira vez: no nativo, o valor de reserva lido de um nome (", ou as linhas") era copiado e, além disso, tirado do nome, que ficava sem quem o liberasse. Corrigido no mesmo envio.

Item 9, módulos:
- A sintaxe ficou `Uso o arquivo "contas.lav".`, no nível de fora, com o caminho a partir da pasta do arquivo que pede. O programa passa a ter tudo o que o arquivo define, sem prefixo.
- O "Para começar" do arquivo trazido não entra, então cada arquivo pode ter o seu e rodar sozinho. O arquivo que só define passa no check e é recusado no run, com uma mensagem própria.
- Um arquivo trazido traz outros e as bibliotecas de que precisa; cada um entra uma vez, e o ciclo se fecha sozinho.
- O erro de um arquivo trazido aparece no arquivo dele (no terminal), ou no pedido que o trouxe (no editor e no JSON). A falha de execução se mostra no arquivo certo também no nativo, que leva os arquivos junto.
- No editor, a definição de um nome de outro arquivo abre o arquivo.
- Testes em tests/modules (fora do corpus de um arquivo só), com a paridade entre o interpretador e o nativo.

Decisões para revisar:
- "Uso o arquivo" e o caminho entre aspas, em vez de um nome de módulo ("Uso o módulo de contas"): diz exatamente o que acontece, e não reserva palavra ("arquivo" continua livre: "Arquivo a carta." é verbo do corpus).
- Vem tudo, sem prefixo e sem "privado". Dois arquivos com o mesmo verbo e a mesma regência não se trazem juntos.
- O rename não atravessa arquivos: ele recusa o nome que vem de fora.
- O editor lê os arquivos trazidos do disco; o que está aberto sem salvar não entra na análise de quem o traz.

Item 10, referências guardadas:
- O particípio do cabeçalho passou a valer também na declaração: "A favorita é a primeira das contas, confiada." faz de "a favorita" uma referência à conta que está na lista, e o que muda por ela muda lá. ", emprestada" é a de leitura, sem cópia.
- O modelo é o do roteiro: lexical. A referência vale da declaração até o fim do bloco, e enquanto isso o dono emprestado não muda nem se entrega, e o confiado nem se lê (código novo, L0407).
- A referência não sai do bloco: não se entrega, não se devolve, não entra em registro nem em lista. É o que dispensa tempos de vida anotados.
- Quatro códigos: L0179 (concordância do particípio com o nome), L0180 (referência com "começa"), L0369 (valor sem lugar) e L0407.

Decisões para revisar:
- A vida da referência é o bloco, e não o último uso (o Rust de hoje usa o último uso). Fica mais simples de explicar, e às vezes obriga a abrir um bloco só para a referência acabar. Não há um jeito de abrir um bloco sem condição; hoje se usa um "Se" que sempre vale.
- O particípio concorda com o nome declarado ("a favorita ..., confiada"), enquanto no cabeçalho ele concorda com o tipo. É a concordância natural de cada frase, mas são duas regras.

Item 11, genéricos e traits, primeira parte (genéricos):
- "qualquer" depois de um nome, no cabeçalho, declara um tipo qualquer: "sendo a lista uma lista de coisas quaisquer emprestada, devolvendo uma lista de coisas". O nome do tipo é livre, e no corpo ele se cita sem "qualquer".
- Cada chamada decide o tipo pelos argumentos. A função é conferida uma vez, como foi escrita, e o programa leva uma instância para cada combinação de tipos; os back-ends não mudaram.
- Com um valor de tipo qualquer a função faz o que vale para todo tipo: entrega, empresta, copia, compara por igualdade, escreve, guarda em lista. "qualquer ordenável" pede um tipo com ordem e libera "maior que".
- Quatro códigos: L0181 e L0182 (a forma), L0370 (tipo que a chamada não decide) e L0371 (tipo sem ordem).

Decisão para revisar: o tipo qualquer não tem marca própria além da palavra "qualquer" na primeira vez; depois é um nome como o de um registro. Se um registro tiver o mesmo nome, o registro vence.

Item 11, segunda parte (traits):
- Na Lavra a regência já é a assinatura; a interface ficou sendo o verbo. Um verbo, com a mesma regência, pode ser definido para vários tipos ("Para descrever a conta, ..." e "Para descrever a forma, ..."), e a chamada usa a definição do tipo do argumento.
- O tipo qualquer pede o verbo com uma oração: "sendo a coisa uma coisa qualquer que se descreve". Dentro da função, o verbo vale para o valor de tipo qualquer; fora, só se chama a função com tipos que se descrevem.
- Não há nome de interface nem declaração separada: quem tem o verbo, tem. Vários verbos se pedem com "que se descreve e que se pesa".
- Quatro códigos: L0372 (verbo de "que se" que não existe), L0373 (definições que não combinam), L0374 (verbo não definido para o tipo) e L0375 (tipo qualquer usado sem pedir o verbo).
- No editor, as definições do mesmo verbo são uma função só, e o rename leva todas e os "que se ..." juntos.

Decisões para revisar:
- Sem interface com nome ("um descritível é quem sabe descrever"). O verbo como interface é mais curto e mais Lavra, mas não agrupa vários verbos sob um nome; se fizer falta, um nome para um conjunto de "que se" é uma extensão pequena.
- As definições do mesmo verbo têm de devolver o mesmo tipo. Um verbo que devolve o próprio tipo qualquer ("copiar a coisa, devolvendo uma coisa") não cabe nessa regra.
- A escolha é pelos tipos escritos: o inteiro escrito num argumento não vira real para casar com uma definição.

Item 12, verbos de primeira classe, que fecha o catálogo:
- O tipo: "um verbo de inteiro para inteiro" (o que recebe depois de "de", o que devolve depois de "para"; "emprestado" no que vem emprestado).
- O valor é o infinitivo, que em português já é o nome do verbo: "transformar as notas com dobrar", "A escolhida, um verbo de inteiro para inteiro, começa em dobrar."
- A chamada é com "invocar", verbo novo da língua: "Invoco a ação com o item.", "o resultado de invocar a junção com o total e o número".
- Guarda-se em nome, lista, registro e mapa; combina com os genéricos ("um verbo de coisa emprestada para texto"). No nativo é o endereço da função, chamado direto.
- Quatro códigos: L0183, L0376, L0377 e L0378.

Decisões para revisar:
- "invocar", e não "aplicar" ou "chamar": os dois são verbos que os programas definem muito ("aplicar o rendimento", "chamar o cliente"), e reservar um deles quebraria programas. "invocar" quase ninguém define.
- Não há fecho: o verbo guardado é sempre uma função com nome, e não leva valores do lugar em que foi escrito.
- Ficam de fora, como valor: a função genérica, a que pode falhar e a que recebe algo confiado.

Com isso o catálogo de features do roteiro (seção 2.4) está feito, menos o nativo em macOS e Windows, que ficou fora por pedido. As pendências que dependem de você continuam as do fim do roteiro: os tokens de publicação e o merge do PR de release.

Um conserto depois do catálogo, achado pelo workflow do site, que vinha falhando:
- O teste da biblioteca de tempo estava quebrado desde que tirei "o segundo de x": eu tinha trocado a conta por um nome "O segundo", que é ordinal, e o programa passou a dar erro de sintaxe. A regravação das expectativas guardou o erro como saída esperada, e nada acusou. O programa foi corrigido, e o teste dos programas certos agora recusa qualquer um que dê erro, com ou sem regravação.
- O teste do playground roda o corpus no navegador, onde não há disco nem relógio do sistema: os programas das bibliotecas de arquivos e de tempo ficam de fora dele.

Dia 10/11/2026, as lacunas que o fecho do catálogo deixou anotadas, registro do Claude:

Três das "decisões para revisar" eram coisas que faltavam, e foram feitas:
- O valor de reserva na leitura de uma chave: "o item "Ana" das idades, ou 0" dá uma cópia do valor, ou o de reserva se a chave falta. Dentro dos argumentos de uma chamada o ", ou" continua sendo da chamada (ali, parênteses). Em lista, nome ou campo é o L0379. De passagem, o valor de reserva que é um nome deixou de contar como entregue para o verificador de posse: os back-ends já o copiavam.
- "folhear o arquivo", na biblioteca de arquivos, dá as linhas, uma por item.
- "talvez" passa a quebrar o ciclo de um registro, como a lista: "o próximo, talvez um elo" é a corrente que acaba em "nada". A explicação do L0302 dizia o contrário, e foi reescrita.

O rename entre arquivos, que só recusava:
- Renomear um nome muda o documento, os arquivos que ele traz e, das pastas abertas no editor, os que trazem o arquivo que define o nome. Vale dos dois lados: a partir de um uso, ou a partir da definição.
- O "Para começar" de um arquivo trazido, que não entra no programa, também usa os nomes: cada arquivo trazido é visto como programa, para esses usos mudarem junto.
- A concordância que o rename não escreve (artigos, particípios) é acertada em cada arquivo, e a conferência final analisa o programa com todos os arquivos como ficaram. Testado com um verbo e com um tipo que troca de gênero ("uma conta" vira "um cofre") nos dois arquivos.
- Limite: o arquivo fora das pastas abertas no editor não é achado.

O depurador no Zed, que tinha ficado de fora na fase 8:
- A extensão registra o lavrac como adaptador de depuração e traduz o "depurar este arquivo" do Zed para a configuração do lavrac dap. O README da extensão mostra a configuração.
- Conferido só pela compilação da extensão para wasm (num contêiner): não havia um Zed nesta máquina para abrir uma sessão de verdade. Fica para você testar.
- Os tokens semânticos no Helix continuam fora: o Helix não os pede ao servidor, e não há o que a configuração possa fazer.

Mais três lacunas da lista do fecho:
- "o segundo de x" voltou à biblioteca de tempo. A colisão com o ordinal ("o segundo das notas") se resolve pelo tipo: o parser lê sempre o ordinal, e a sema, vendo que o que vem depois é o nome de um inteiro num programa que traz a biblioteca, troca pela parte do relógio. Vale para um nome; para uma conta, guarde num nome antes.
- O rename procura quem traz o arquivo também pela pasta do documento e pela de cima, para o arquivo aberto sem pasta no editor. O limite que sobra é o de qualquer rename: não há índice de todos os arquivos Lavra da máquina.
- O depurador do Zed ganhou conferência além da compilação: o registro da extensão foi comparado com a documentação do Zed, e um teste abre uma sessão no lavrac dap com a configuração exata que a extensão monta (com o "request" e o "stopOnEntry"). Continua faltando ver a sessão num Zed aberto, que esta máquina não tem.

O que fica de fora por não depender do projeto: os tokens semânticos no Helix (o Helix não os pede a servidor nenhum).

