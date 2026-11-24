// sec-async-generator-function-definitions-static-semantics-early-errors: SyntaxError "yield may not be an identifier in a generator function"
var g = async function* yield() {};
