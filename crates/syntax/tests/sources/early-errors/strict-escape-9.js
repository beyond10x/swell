// sec-string-literals-early-errors: SyntaxError "\8 or \9 escape sequence in strict mode code"
"use strict";
x = { "\9": 1 };
