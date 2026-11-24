// sec-async-generator-function-definitions-static-semantics-early-errors: SyntaxError "await in the parameters of an async function"
x = { async *m(a = await 1) {} };
