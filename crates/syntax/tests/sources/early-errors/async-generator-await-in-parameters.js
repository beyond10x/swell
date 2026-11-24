// sec-async-generator-function-definitions-static-semantics-early-errors: SyntaxError "await in the parameters of an async function"
var g = async function* (x = await 1) {};
