// sec-async-generator-function-definitions-static-semantics-early-errors: SyntaxError "duplicate parameter name a in strict mode code"
async function* g(a, a) {
  "use strict";
}
