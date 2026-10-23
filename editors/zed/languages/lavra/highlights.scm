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

(flow_verb) @keyword.return

[
  (declaration_verb)
  (mutation_verb)
  (signature_keyword)
  (position)
  (output)
] @keyword

(control_keyword) @keyword.repeat

(conditional_keyword) @keyword.conditional

(repeat_keyword) @keyword.repeat

(function_keyword) @keyword.function

(import_keyword) @keyword.import

(definition_keyword) @keyword.type

(ownership_keyword) @keyword.modifier

(primitive_type) @type.builtin

(nominal) @function.builtin

(literal) @constant.builtin

(anaphora) @variable.special

; Nomes e símbolos.

(identifier) @variable

(symbol) @variable

; O artigo sozinho acompanha a palavra que ele determina: "o resultado", "a soma de",
; "a primeira das notas".

((article) @variable.special
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
  (article) @keyword.import)

; O verbo de uma oração: no começo dela, depois de "e", depois do sujeito de um predicado
; ("Se o saldo exceder o teto", "Se 120 exceder o teto"), depois de "for" (o predicado
; adjetivo: "Se o número for par") e depois de "o resultado de".

(sentence
  .
  (identifier) @function.call)

((conjunction)
  .
  (identifier) @function.call)

([
  (name)
  (symbol)
  (number)
  (string)
  (character)
  (parenthesized)
]
  .
  (identifier) @function.call)

((copula)
  .
  (identifier) @function.call)

((anaphora)
  .
  (preposition)
  .
  (identifier) @function.call)

; O tipo ou o caso depois do artigo indefinido: "uma conta", "um círculo".

((indefinite_article)
  .
  (identifier) @type)

; "Senão, se" encadeia uma condição; no cabeçalho, "sendo" dá os tipos dos parâmetros.

(else_statement
  (control_keyword) @keyword.conditional)

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
  name: (identifier) @type.definition)

(variant
  name: (identifier) @variant)

; Os campos de um registro e de um caso.

(type_definition
  (block
    (sentence
      .
      (name
        (identifier) @property))))

(type_definition
  (block
    (sentence
      .
      (symbol) @property)))

(variant
  (name
    (identifier) @property))

(import
  library: (_) @type)

; Valores e o resto.

(number) @number

(string) @string

(escape_sequence) @string.escape

(character) @string.special

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
