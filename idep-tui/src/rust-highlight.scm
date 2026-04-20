; Basic keywords
"fn" @keyword
"let" @keyword
"return" @keyword
"if" @keyword
"else" @keyword
"for" @keyword
"while" @keyword
"struct" @keyword
"impl" @keyword
"pub" @keyword
"use" @keyword
"mod" @keyword

; Literals
(string_literal) @string
(char_literal) @string
(integer_literal) @number
(float_literal) @number
(boolean_literal) @boolean

; Comments
(line_comment) @comment
(block_comment) @comment

; Function definitions
(function_item
  name: (identifier) @function)

; Function calls
(call_expression
  function: (identifier) @function.call)

; Identifiers
(identifier) @variable
(type_identifier) @type
