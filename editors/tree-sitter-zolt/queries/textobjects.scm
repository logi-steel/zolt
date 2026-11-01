; Obiekty tekstowe dla Helixa: maf / mif zaznacza funkcję albo metodę,
; mat / mit szablon, ]f skacze do następnej funkcji.
(function_definition
  body: (block) @function.inside) @function.around

(method_definition
  body: (block) @function.inside) @function.around

(template
  body: (template_body) @class.inside) @class.around

(parameter) @parameter.inside

(comment) @comment.inside
(comment)+ @comment.around
