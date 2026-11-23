// sec-generator-function-definitions-static-semantics-early-errors: SyntaxError "yield expression in the parameters of a generator function"
function* g(x = yield) {}
