// sec-for-in-and-for-of-statements-static-semantics-early-errors: SyntaxError "let may not start the target of a for-of statement"
// sloppy-only
async function f(o) { for await (let.x of o) ; }
