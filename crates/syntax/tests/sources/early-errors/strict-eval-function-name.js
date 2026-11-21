// sec-function-definitions-static-semantics-early-errors: SyntaxError "eval may not be bound in strict mode code"
function eval() {
  "use strict";
}
