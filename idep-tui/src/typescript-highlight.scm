[
  "as"
  "async"
  "await"
  "break"
  "const"
  "continue"
  "debugger"
  "default"
  "delete"
  "do"
  "else"
  "export"
  "extends"
  "finally"
  "for"
  "from"
  "function"
  "get"
  "if"
  "import"
  "in"
  "instanceof"
  "let"
  "new"
  "of"
  "return"
  "set"
  "static"
  "switch"
  "target"
  "throw"
  "try"
  "typeof"
  "var"
  "void"
  "while"
  "with"
  "yield"
] @keyword

(function_declaration
  name: (identifier) @function)

(call_expression
  function: (identifier) @function.call)

(string) @string
(template_string) @string
(regex) @string
(number) @number
(boolean) @boolean

(comment) @comment

(type_identifier) @type
(predefined_type) @type.builtin

(property_identifier) @property

(identifier) @variable

(call_expression
  function: (identifier) @tag)
