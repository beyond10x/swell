// sec-async-function-definitions-static-semantics-early-errors: SyntaxError "await in the parameters of an async function"
async function f(x = await 1) {}
