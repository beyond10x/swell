// sec-for-in-and-for-of-statements: SyntaxError "unexpected token await"
function f(o) { for await (x of o) ; }
