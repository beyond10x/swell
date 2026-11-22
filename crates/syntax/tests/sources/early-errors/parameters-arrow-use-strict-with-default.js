// sec-arrow-function-definitions-static-semantics-early-errors: SyntaxError "use strict directive in a function whose parameter list is not simple"
var f = (a = 1) => { "use strict"; };
