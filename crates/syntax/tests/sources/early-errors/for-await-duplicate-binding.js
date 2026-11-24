// sec-for-in-and-for-of-statements-static-semantics-early-errors: SyntaxError "duplicate lexical declaration of a"
async function f(o) { for await (let { a, b: a } of o) ; }
