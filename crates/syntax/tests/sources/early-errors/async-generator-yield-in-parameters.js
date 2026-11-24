// sec-async-generator-function-definitions-static-semantics-early-errors: SyntaxError "yield expression in the parameters of a generator function"
async function* g(x = yield) {}
