// sec-for-statement-static-semantics-early-errors: SyntaxError "lexical declaration of i conflicts with a var or function declaration"
for (let i = 0; i < 1; i++) { var i; }
