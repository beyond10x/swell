// sec-identifiers-static-semantics-early-errors: SyntaxError "arguments may not be bound in strict mode code"
function f(arguments) {
  "use strict";
}
