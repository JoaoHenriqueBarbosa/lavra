# Notas do projeto Lavra

Notas do projeto. As três primeiras descrevem a Lavra como ela é hoje (o compilador em
Rust); as demais são o contexto do protótipo (`draft/`) e das conversas em que a Lavra
nasceu (claude.ai, 2026-10-21).

| Arquivo | Conteúdo |
|---|---|
| [lavra.md](lavra.md) | **A língua**: declaração, mutação, tipos, registros, enumerações, funções e regência, posse e empréstimo, talvez e falha, condições, expressões. |
| [compiler.md](compiler.md) | **O compilador**: pipeline, nativo com Cranelift e runtime, testes (golden, paridade, valgrind). |
| [morphology.md](morphology.md) | **A morfologia**: as tabelas conferidas contra o dicionário, as decisões sobre os casos de borda e o que ficou pendente. |
| [origin.md](origin.md) | Como a Lavra surgiu: o pedido original, as decisões de design e a ordem em que as coisas foram feitas. |
| [language.md](language.md) | A linguagem como estava no protótipo: morfologia, sintaxe, tipos, semântica, erros. Inclui o que surgiu na implementação e não estava no design inicial. |
| [draft-architecture.md](draft-architecture.md) | Como o draft é construído: arquivos, pipeline do compilador, interpretador, gerador x86-64, runtime C, editor HTML, CLI. |
| [draft-status.md](draft-status.md) | O que foi verificado em 2026-10-22, o patch que só existe na raiz do `draft/`, limitações e bugs conhecidos. |
| [applications.md](applications.md) | As oito aplicações discutidas, o roadmap sugerido e as extensões de gramática pendentes. |
| [poems.md](poems.md) | Os três programas-poema, os sentidos duplos e o programa *ab urbe condita* que ficou só na conversa. |

Fonte primária de tudo: `draft/` (código) e as duas conversas coladas pelo user na
sessão de 2026-10-22.
