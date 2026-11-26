// sec-class-definitions-static-semantics-early-errors: SyntaxError "lexical declaration of x conflicts with a var or function declaration"
class C { static { var x; let x; } }
