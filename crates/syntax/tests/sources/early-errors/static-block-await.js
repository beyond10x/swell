// sec-class-definitions-static-semantics-early-errors: SyntaxError "await may not be an identifier in a class static block"
class C { static { await; } }
