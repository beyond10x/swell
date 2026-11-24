// sec-async-arrow-function-definitions-static-semantics-early-errors: SyntaxError "duplicate parameter name a in an arrow function"
var f = async (a, a) => 1;
