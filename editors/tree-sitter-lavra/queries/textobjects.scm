; Objetos de texto, com os nomes do Helix e do Zed ("inside" e "around"); o Neovim
; (nvim-treesitter-textobjects) recebe uma cópia com "inner" e "outer".

(function_definition
  (block) @function.inside) @function.around

(main_definition
  (block) @function.inside) @function.around

(type_definition
  (block) @class.inside) @class.around

(parameter) @parameter.inside

(comment) @comment.inside

(comment)+ @comment.around
