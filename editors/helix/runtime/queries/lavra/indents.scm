; Recuo automático no Helix: depois dos dois-pontos o bloco entra recuado, e o período que
; continua na linha de baixo (enjambement) também.

[
  (if_statement)
  (else_statement)
  (while_statement)
  (repeat_statement)
  (for_statement)
  (match_statement)
  (match_arm)
  (function_definition)
  (main_definition)
  (type_definition)
  (sentence)
  (parenthesized)
] @indent

; Enquanto se digita, a construção vai até a última linha mais recuada que ela.

[
  (if_statement)
  (else_statement)
  (while_statement)
  (repeat_statement)
  (for_statement)
  (match_statement)
  (match_arm)
  (function_definition)
  (main_definition)
  (type_definition)
] @extend

; Depois de devolver, desistir, parar ou pular, a linha seguinte sai do bloco.

(sentence
  .
  (flow_verb)) @extend.prevent-once

")" @outdent
