// sec-string-literals-early-errors: SyntaxError "legacy octal escape sequence in strict mode code"
function f() {
  "\1";
  "use strict";
}
