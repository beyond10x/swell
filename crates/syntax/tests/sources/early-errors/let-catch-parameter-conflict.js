// sec-try-statement-static-semantics-early-errors: SyntaxError "lexical declaration of e conflicts with the catch parameter"
try {} catch (e) { let e; }
