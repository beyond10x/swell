// sec-class-definitions-static-semantics-early-errors: SyntaxError "super call outside a derived class constructor"
class A extends B { m() { super(); } }
