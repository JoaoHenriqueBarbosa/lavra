; Gerado de editors/tree-sitter-lavra/queries/highlights.scm por scripts/sync-queries.js: não edite aqui.

; As palavras da língua, pelo papel que o léxico dá a cada uma.

(article) @variable

(indefinite_article) @keyword

[
  (contraction)
  (preposition)
  (conjunction)
  (copula)
  (operator)
  (comparison)
] @keyword.operator

(core_verb) @function.builtin

(flow_verb) @keyword.control.return

[
  (declaration_verb)
  (mutation_verb)
  (signature_keyword)
  (position)
  (output)
] @keyword

(control_keyword) @keyword.control.repeat

(conditional_keyword) @keyword.control.conditional

(repeat_keyword) @keyword.control.repeat

(function_keyword) @keyword.function

(import_keyword) @keyword.control.import

(definition_keyword) @keyword.storage.type

(ownership_keyword) @keyword.storage.modifier

(primitive_type) @type.builtin

(nominal) @function.builtin

(literal) @constant.builtin

(anaphora) @variable.builtin

; Nomes e símbolos.

(identifier) @variable

(symbol) @variable

; O artigo sozinho acompanha a palavra que ele determina: "o resultado", "a soma de",
; "a primeira das notas".

((article) @variable.builtin
  .
  (anaphora))

((article) @function.builtin
  .
  (nominal))

((article) @keyword
  .
  (position))

; O "a" de "igual a" e "iguais a" é a preposição da locução; o de "Uso a biblioteca", parte
; da frase.

((comparison)
  .
  (article) @keyword.operator)

(import
  (article) @keyword.control.import)

; O verbo de uma oração: no começo dela, depois de "e", depois do sujeito de um predicado
; ("Se o saldo exceder o teto", "Se 120 exceder o teto"), depois de "for" (o predicado
; adjetivo: "Se o número for par") e depois de "o resultado de".

(sentence
  .
  (identifier) @function)

((conjunction)
  .
  (identifier) @function)

([
  (name)
  (symbol)
  (number)
  (string)
  (character)
  (parenthesized)
]
  .
  (identifier) @function)

((copula)
  .
  (identifier) @function)

((anaphora)
  .
  (preposition)
  .
  (identifier) @function)

; O tipo ou o caso depois do artigo indefinido: "uma conta", "um círculo".

((indefinite_article)
  .
  (identifier) @type)

; "Senão, se" encadeia uma condição; no cabeçalho, "sendo" dá os tipos dos parâmetros.

(else_statement
  (control_keyword) @keyword.control.conditional)

(function_definition
  (control_keyword) @keyword)

; As definições.

(function_definition
  name: (_) @function)

(main_definition
  name: (_) @function)

; A 1ª pessoa declarada: "Para trovejar (trovejo) ...".

(function_definition
  (parenthesized
    (identifier) @function))

(parameter
  (article) @variable.parameter)

(parameter
  (identifier) @variable.parameter)

(function_definition
  (symbol) @variable.parameter)

(type_definition
  name: (identifier) @type)

(variant
  name: (identifier) @type.enum.variant)

; Os campos de um registro e de um caso.

(type_definition
  (block
    (sentence
      .
      (name
        (identifier) @variable.other.member))))

(type_definition
  (block
    (sentence
      .
      (symbol) @variable.other.member)))

(variant
  (name
    (identifier) @variable.other.member))

(import
  library: (_) @namespace)

; Valores e o resto.

(number) @constant.numeric

(string) @string

(escape_sequence) @constant.character.escape

(character) @constant.character

(comment) @comment

[
  "."
  ";"
  ","
  ":"
] @punctuation.delimiter

[
  "("
  ")"
] @punctuation.bracket
