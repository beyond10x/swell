// sec-for-in-and-for-of-statements: SyntaxError "unexpected token await"
async function f(o) { for \u0061wait (x of o) ; }
