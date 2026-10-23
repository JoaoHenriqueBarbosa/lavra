; Gerado de editors/tree-sitter-lavra/queries/locals.scm por scripts/sync-queries.js: não edite aqui.

[
  (function_definition)
  (main_definition)
  (block)
] @local.scope

; Os parâmetros: os nomes do cabeçalho e os símbolos ("Para elevar x a n").

(parameter
  (identifier) @local.definition.variable.parameter)

(function_definition
  (symbol) @local.definition.variable.parameter)

; A declaração: o nome que abre a oração com "é", "são", "começa" ("O saldo começa em 0.",
; "A taxa, um inteiro, é 3.").

(sentence
  .
  (name
    (identifier) @local.definition.variable)
  (declaration_verb))

; O item de "Para cada": "Para cada nota das notas", "Para cada i de 1 até 10".

(for_statement
  .
  (repeat_keyword)
  .
  (repeat_keyword)
  .
  [
    (identifier)
    (symbol)
  ] @local.definition.variable)

(function_definition
  name: (identifier) @local.definition.function)

(type_definition
  name: (identifier) @local.definition.type)

(name
  (identifier) @local.reference)

(symbol) @local.reference
