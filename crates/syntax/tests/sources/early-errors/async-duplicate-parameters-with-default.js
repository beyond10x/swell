// sec-async-function-definitions-static-semantics-early-errors: SyntaxError "duplicate parameter name a in a parameter list that is not simple"
async function f(a, a = 0) {}
