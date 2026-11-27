// sec-static-semantics-allprivateidentifiersvalid: SyntaxError "private name #y is not declared in an enclosing class"
class C { #x; m() { return this.#y; } }
