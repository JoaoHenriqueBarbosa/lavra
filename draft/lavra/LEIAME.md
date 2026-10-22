# Lavra

Linguagem de programação de baixo nível escrita em português gramatical.

- `lavra.js` — léxico, morfologia, análise sintática com verificação de concordância,
  regência, modo verbal e tipos, e a máquina que executa o programa.
- `x86.js` — gerador de assembly x86-64 (sintaxe Intel, convenção System V).
- `runtime.c` — biblioteca de execução dos programas compilados (entrada, saída, memória, erros).
- `lavrac.js` — linha de comando.
- `lavra.html` — o editor completo, abre em qualquer navegador sem instalar nada.
- `exemplos/` — programas de exemplo.

Uso (precisa de Node; para gerar executável, gcc em Linux x86-64):

    node lavrac.js exemplos/lista2.lav          # interpreta
    node lavrac.js -s exemplos/lista2.lav       # gera exemplos/lista2.s
    node lavrac.js -c exemplos/lista2.lav       # gera o executável exemplos/lista2
    ./exemplos/lista2

Todos os exemplos foram compilados para x86-64 e a saída do executável nativo
conferida contra a do interpretador.
