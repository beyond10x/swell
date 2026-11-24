// sec-for-in-and-for-of-statements-static-semantics-early-errors: SyntaxError "invalid assignment target"
async function f(o) { for await (a, b of o) ; }
