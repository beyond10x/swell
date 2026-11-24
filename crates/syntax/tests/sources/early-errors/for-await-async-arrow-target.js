// sec-for-in-and-for-of-statements: SyntaxError "unexpected token =>"
async function f(o) { for await (async of => {} of o) ; }
