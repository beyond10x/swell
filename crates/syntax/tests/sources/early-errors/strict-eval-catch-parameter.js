// sec-identifiers-static-semantics-early-errors: SyntaxError "eval may not be bound in strict mode code"
"use strict";
try {} catch (eval) {}
