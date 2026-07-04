["import" "export" "func" "mut" "null" "record"] @keyword

["i8" "u8" "i32" "u32" "bool"] @type.builtin

(ident) @variable
(number) @number
(func_sig name: (ident) @function)
(param name: (ident) @variable.parameter)
(linear_field name: (ident) @property)
(comment) @comment
(_bool) @constant.builtin

["{" "}" "(" ")"] @punctuation.bracket

[
  ","
  ":"
  ";"
  "->"
] @punctuation.delimiter

["+" "*" "<" ">" "&&"] @operator

;;(ERROR) @error
