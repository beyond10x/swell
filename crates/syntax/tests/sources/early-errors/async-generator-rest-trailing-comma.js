// sec-async-generator-function-definitions: SyntaxError "a rest parameter must be the last parameter"
async function* g(...a,) {}
