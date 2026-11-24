// sec-async-generator-function-definitions-static-semantics-early-errors: SyntaxError "duplicate parameter name a in a parameter list that is not simple"
async function* g(a, a = 0) {}
