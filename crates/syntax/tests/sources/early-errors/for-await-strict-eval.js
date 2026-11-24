// sec-for-in-and-for-of-statements-static-semantics-early-errors: SyntaxError "eval may not be assigned in strict mode code"
"use strict";
async function f(o) { for await (eval of o) ; }
