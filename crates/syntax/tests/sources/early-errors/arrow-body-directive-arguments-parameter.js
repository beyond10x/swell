// sec-identifiers-static-semantics-early-errors: SyntaxError "arguments may not be bound in strict mode code"
var f = (arguments) => { "use strict"; };
