; Gerado de editors/tree-sitter-lavra/queries/indents.scm por scripts/sync-queries.js: não edite aqui.

; Depois dos dois-pontos, o bloco: a linha nova já entra recuada, mesmo antes de o bloco
; existir na árvore.

((if_statement) @indent.begin
  (#set! indent.immediate 1))

((else_statement) @indent.begin
  (#set! indent.immediate 1))

((while_statement) @indent.begin
  (#set! indent.immediate 1))

((repeat_statement) @indent.begin
  (#set! indent.immediate 1))

((for_statement) @indent.begin
  (#set! indent.immediate 1))

((match_statement) @indent.begin
  (#set! indent.immediate 1))

((match_arm) @indent.begin
  (#set! indent.immediate 1))

((function_definition) @indent.begin
  (#set! indent.immediate 1))

((main_definition) @indent.begin
  (#set! indent.immediate 1))

((type_definition) @indent.begin
  (#set! indent.immediate 1))

; O período que continua na linha de baixo (enjambement) e o parêntese aberto.

(sentence) @indent.begin

((parenthesized) @indent.align
  (#set! indent.open_delimiter "(")
  (#set! indent.close_delimiter ")"))

; "Senão" e os casos de "Conforme" voltam ao nível do bloco que continuam.

[
  (else_statement)
  (match_arm)
] @indent.branch

(parenthesized
  ")" @indent.end)
