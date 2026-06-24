["import" "export" "func"] @keyword

(_val_type) @type
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
