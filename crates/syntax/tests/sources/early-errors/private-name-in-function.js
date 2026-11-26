// sec-static-semantics-allprivateidentifiersvalid: SyntaxError "private name #x is not declared in an enclosing class"
function f(o) { return o.#x; }
