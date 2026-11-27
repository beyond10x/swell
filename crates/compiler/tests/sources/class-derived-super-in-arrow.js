// sec-super-keyword-runtime-semantics-evaluation: an arrow reaches the constructor's `this`, so it lives in the function environment; the arrow binds it and reads it there
class B extends A { constructor() { var f = () => super(); f(); return this; } }
