// sec-for-in-and-for-of-statements: SyntaxError "unexpected token x"
async function f() { for await (let of x) ; }
