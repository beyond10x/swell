// sec-function-definitions-static-semantics-early-errors: SyntaxError "duplicate parameter name a in strict mode code"
var f = function (a, a) {
  "use strict";
};
