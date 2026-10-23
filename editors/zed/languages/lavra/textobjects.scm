; Gerado de editors/tree-sitter-lavra/queries/textobjects.scm por scripts/sync-queries.js: não edite aqui.

(function_definition
  (block) @function.inside) @function.around

(main_definition
  (block) @function.inside) @function.around

(type_definition
  (block) @class.inside) @class.around

(parameter) @parameter.inside

(comment) @comment.inside

(comment)+ @comment.around
