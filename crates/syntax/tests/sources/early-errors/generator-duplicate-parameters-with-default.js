// sec-generator-function-definitions-static-semantics-early-errors: SyntaxError "duplicate parameter name a in a parameter list that is not simple"
function* g(a, a = 0) {}
