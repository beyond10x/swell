// sec-for-in-and-for-of-statements: SyntaxError "unexpected token in"
async function f() { for await (x in y) ; }
