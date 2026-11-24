// sec-async-generator-function-definitions-static-semantics-early-errors: SyntaxError "await may not be an identifier in an async function"
var g = async function* await() {};
