// sec-parameter-lists-static-semantics-early-errors: SyntaxError "duplicate parameter name a in strict mode code"
"use strict";
function f(a, b, a) {}
