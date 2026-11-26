// sec-async-function-definitions-static-semantics-early-errors: SyntaxError "super property outside a method"
async function f() { await super["x"]; }
