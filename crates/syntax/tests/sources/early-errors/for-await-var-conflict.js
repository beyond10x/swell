// sec-for-in-and-for-of-statements-static-semantics-early-errors: SyntaxError "lexical declaration of k conflicts with a var or function declaration"
async function f() { for await (const k of []) { var k; } }
