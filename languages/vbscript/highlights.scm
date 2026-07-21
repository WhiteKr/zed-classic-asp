(comment) @comment
(string) @string
(number) @number
(boolean) @boolean
(constant) @constant.builtin
(keyword) @keyword
(operator) @operator
(builtin_object) @variable.special

((identifier) @function.builtin
  (#match? @function.builtin "^(?i)(abs|array|asc|atn|cbool|cbyte|ccur|cdate|cdbl|chr|cint|clng|cos|csng|cstr|date|dateadd|datediff|datepart|dateserial|datevalue|day|escape|eval|exp|filter|fix|formatcurrency|formatdatetime|formatnumber|formatpercent|getlocale|getobject|getref|hex|hour|inputbox|instr|instrrev|int|isarray|isdate|isempty|isnull|isnumeric|isobject|join|lbound|lcase|left|len|loadpicture|log|ltrim|mid|minute|month|monthname|msgbox|now|oct|replace|rgb|right|rnd|round|rtrim|scriptengine|second|setlocale|sgn|sin|space|split|sqr|strcomp|string|strreverse|tan|time|timer|timeserial|timevalue|trim|typename|ubound|ucase|unescape|vartype|weekday|weekdayname|year|createobject)$"))

((identifier) @constant.builtin
  (#match? @constant.builtin "^(?i)vb[a-z0-9]+$"))

(sub_definition
  name: (identifier) @function)

(function_definition
  name: (identifier) @function)

(class_definition
  name: (identifier) @type)

(property_definition
  name: (identifier) @property)
