["import" "export" "func"] @keyword

(type) @type
(number) @number
(func_sig name: (ident) @function)
(comment) @comment

["{" "}"] @punctuation.bracket

["(" ")"] @punctuation.bracket

[
  ","
  "->"
] @punctuation.delimiter

;;(ERROR) @error
