// sec-switch-statement-static-semantics-early-errors: SyntaxError "duplicate lexical declaration of a"
switch (0) { case 0: let a; break; default: let a; }
