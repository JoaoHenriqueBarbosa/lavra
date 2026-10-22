Dia 22/10/2026, reescrita da Lavra em Rust, mensagem final do Claude, depois de aproximadamente 60 minutos de trabalho contínuo:

A Lavra está implementada do início ao fim do plano, da F0 à F4. Programas certos e errados rodam no interpretador e viram executáveis nativos pelo Cranelift, com saída idêntica nos dois.

O que funciona
- A língua inteira do plano:
  - declarações com "é" (fixo) e "começa em" (variável), mutação com o dado como sujeito ("vira", "cresce", "ganha", "perde", "Tomo");
  - funções com regência, predicados no subjuntivo, o fato de e conversão com como;
  - registros, listas, enumerações com Conforme exaustivo;
  - talvez/Se houver, e falha com salvo uma falha, Desisto com e , ou desisto.
- O verificador de posse responde em português: "A carta foi entregue a "arquivar" na linha 6 e não volta", com a dica de pedir emprestado ou entregar uma cópia. Ele segue o programa pelos caminhos de Se e pelas voltas dos laços, e acusa o duplo confiado na mesma chamada e a coleção que muda dentro do próprio Para cada.
- Desempenho: o nativo foi 82 vezes mais rápido que o interpretador (0,35 s contra 28,7 s para contar os primos até 2 milhões), com checagem de estouro em toda operação.
- "Lavra dos dias" reescrito na língua nova: a saída é idêntica à do draft. Perdeu o duplo sentido de "endereço" e ganhou "uma data com o dia o sol, o mês a lua e o ano o tempo".

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

