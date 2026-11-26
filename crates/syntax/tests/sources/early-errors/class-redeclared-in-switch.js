// sec-switch-statement-static-semantics-early-errors: SyntaxError "duplicate lexical declaration of A"
switch (0) { case 0: class A {} break; default: class A {} }
