// sec-static-semantics-allprivateidentifiersvalid: SyntaxError "private name #x is not declared in an enclosing class"
class C extends (o => o.#x) { #x; }
