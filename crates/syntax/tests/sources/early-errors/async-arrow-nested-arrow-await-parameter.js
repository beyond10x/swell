// sec-identifiers-static-semantics-early-errors: SyntaxError "await may not be an identifier in an async function"
var f = async (a = (await) => {}) => {};
