// sec-async-generator-function-definitions-static-semantics-early-errors: SyntaxError "use strict directive in a function whose parameter list is not simple"
async function* g(a = 0) {
  "use strict";
}
