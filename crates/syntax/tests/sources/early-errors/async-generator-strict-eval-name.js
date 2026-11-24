// sec-async-generator-function-definitions-static-semantics-early-errors: SyntaxError "eval may not be bound in strict mode code"
"use strict";
var g = async function* eval() {};
