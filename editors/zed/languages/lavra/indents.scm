; Recuo automático no Zed: o corpo das construções, o período que continua na linha de baixo
; e os parênteses. O recuo depois dos dois-pontos, enquanto o bloco ainda não existe, vem do
; `increase_indent_pattern` do config.toml.

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
] @indent

(parenthesized
  ")" @end) @indent

; Os blocos que "Senão" e "Sendo" continuam (`decrease_indent_patterns` do config.toml).

(if_statement) @start.if

(else_statement) @start.else

(match_arm) @start.arm
