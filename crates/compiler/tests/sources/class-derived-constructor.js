// sec-super-keyword-runtime-semantics-evaluation: the heritage in the class scope, a derived F, `this` a checked local made uninitialized before the parameters, each SuperCall bound and initialized, KeepThis before every Return
class B extends A { constructor(x, ...r) { super(x, ...r); this.y = x; if (x) return; } }
