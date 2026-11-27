// sec-runtime-semantics-classdefinitionevaluation: normal "A,1,constructor;m,true,get g"
class A {
  constructor(x) { this.x = x; }
  m() { return this.x; }
  get g() { return 0; }
  static n() { return "A"; }
}
var a = new A(1);
[A.n(), a.m(), Object.getOwnPropertyNames(A.prototype).slice(0, 2).join(";"), a instanceof A,
 Object.getOwnPropertyDescriptor(A.prototype, "g").get.name].join();
