// sec-generator-function-definitions-static-semantics-early-errors: SyntaxError "duplicate parameter name a in strict mode code"
function* g(a, a) {
  "use strict";
}
