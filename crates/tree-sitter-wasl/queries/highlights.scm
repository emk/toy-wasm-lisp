["import" "export" "func" "mut" "null" "record"] @keyword

["i8" "u8" "i32" "u32"] @type.builtin

(ident) @variable
(number) @number
(func_sig name: (ident) @function)
(param name: (ident) @variable.parameter)
(linear_field name: (ident) @property)
(comment) @comment

["{" "}" "(" ")"] @punctuation.bracket

[
  ","
  ":"
  ";"
  "->"
] @punctuation.delimiter

["+" "*"] @operator

;;(ERROR) @error
