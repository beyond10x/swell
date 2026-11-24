// sec-for-in-and-for-of-statements: SyntaxError "initializer in a for-of head"
async function f() { for await (var a = 0 of b) ; }
