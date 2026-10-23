; Gerado de editors/tree-sitter-lavra/queries/textobjects.scm por scripts/sync-queries.js: não edite aqui.

(function_definition
  (block) @function.inner) @function.outer

(main_definition
  (block) @function.inner) @function.outer

(type_definition
  (block) @class.inner) @class.outer

(parameter) @parameter.inner

(comment) @comment.inner

(comment)+ @comment.outer
