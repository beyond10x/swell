// sec-for-in-and-for-of-statements-static-semantics-early-errors: SyntaxError "let is not a valid lexically bound name"
async function f() { for await (let let of []) ; }
