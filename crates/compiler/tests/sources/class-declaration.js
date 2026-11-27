// sec-runtime-semantics-classdefinitionevaluation: the prototype, F from the constructor's template, the methods in source order on the prototype or on F, then the own name
class A { constructor(x) { this.x = x; } m() { return A; } get g() { return 1; } static s() {} }
