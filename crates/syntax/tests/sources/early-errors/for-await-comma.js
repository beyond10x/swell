// sec-for-in-and-for-of-statements: SyntaxError "unexpected token ,"
async function f() { for await (x of a, b) ; }
