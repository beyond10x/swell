// sec-async-function-definitions-static-semantics-early-errors: SyntaxError "duplicate parameter name a in strict mode code"
async function f(a, a) {
  "use strict";
}
