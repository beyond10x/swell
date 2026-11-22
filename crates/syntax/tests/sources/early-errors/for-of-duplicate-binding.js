// sec-for-in-and-for-of-statements-static-semantics-early-errors: SyntaxError "duplicate lexical declaration of a"
for (let { a, b: a } of o) {}
