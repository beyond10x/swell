// sec-try-statement-static-semantics-early-errors: SyntaxError "lexical declaration of a conflicts with a var or function declaration"
try {} catch ({ a }) { var a; }
