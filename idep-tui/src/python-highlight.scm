[
  "and"
  "as"
  "assert"
  "async"
  "await"
  "break"
  "class"
  "continue"
  "def"
  "del"
  "elif"
  "else"
  "except"
  "finally"
  "for"
  "from"
  "global"
  "if"
  "import"
  "in"
  "is"
  "lambda"
  "nonlocal"
  "not"
  "or"
  "pass"
  "raise"
  "return"
  "try"
  "while"
  "with"
  "yield"
] @keyword

(function_definition
  name: (identifier) @function)

(call
  function: (identifier) @function.call)

(string) @string
(escape_sequence) @string
(integer) @number
(float) @number
(boolean) @boolean
(comment) @comment

(identifier) @variable
(type) @type

(decorator) @attribute

(class_definition
  name: (identifier) @type)

(class_definition
  superclasses: (argument_list
    (identifier) @type))
