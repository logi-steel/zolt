; Kolory składni Zolta. W Helixie późniejszy wzorzec wygrywa z wcześniejszym,
; więc najpierw ogólne (identifier), potem szczegółowe (funkcje, pola, sys::).

(identifier) @variable

(comment) @comment

(number) @constant.numeric
(typed_number) @constant.numeric
(boolean) @constant.builtin.boolean

(string) @string
(list_item) @string
(escape_sequence) @constant.character.escape
(interpolation) @variable

"fn" @keyword.function
"retrn" @keyword.control.return
["if" "else" "elseif"] @keyword.control.conditional
["loop" "while" "times" "inf" "end"] @keyword.control.repeat
"has" @keyword.operator
"let" @keyword.storage
["&_" "&"] @keyword.storage.modifier
"use" @keyword.control.import
"lst" @type.builtin

["=" "<" ">" "<=" ">=" "+" "-" "*" "/" "%" "<>" "->" "=>"] @operator

["(" ")" "[" "]" "{" "}"] @punctuation.bracket
["," "." ":" "::"] @punctuation.delimiter

(parameter name: (identifier) @variable.parameter)
(use_clause (scoped (identifier) @namespace))

(scoped scope: (identifier) @namespace)
(scoped name: (identifier) @function.builtin)
((scoped name: (identifier) @variable.builtin)
  (#match? @variable.builtin "^read[0-9]+$"))

(call function: (identifier) @function)
(function_definition name: (identifier) @function)

(template name: (identifier) @type)
(field_definition name: (identifier) @variable.other.member)
(method_definition name: (identifier) @function.method)
(member name: (identifier) @variable.other.member)
((member name: (identifier) @function.builtin)
  (#match? @function.builtin "^(add|del|chnm)$"))
(method_call name: (identifier) @function.method)
