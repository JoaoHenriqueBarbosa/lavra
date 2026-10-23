; As definições e as chamadas, para o `tree-sitter tags` (navegação de código).

(function_definition
  name: (_) @name) @definition.function

(main_definition
  name: (_) @name) @definition.function

(type_definition
  name: (identifier) @name) @definition.class

(variant
  name: (identifier) @name) @definition.constant

(sentence
  .
  (identifier) @name) @reference.call
