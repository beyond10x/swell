// sec-for-in-and-for-of-statements-static-semantics-early-errors: SyntaxError "eval may not be assigned in strict mode code"
"use strict";
for (eval in o) {}
