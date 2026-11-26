// sec-class-definitions-static-semantics-early-errors: SyntaxError "super call outside a derived class constructor"
// Today's answer: every SuperCall is an early error until story:derived-classes-and-super-call, which makes this source parse.
class A extends B { constructor() { super(); } }
