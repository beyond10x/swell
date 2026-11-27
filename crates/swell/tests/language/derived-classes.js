// sec-runtime-semantics-classdefinitionevaluation: normal "B:1,2,A.m>B,true,true,ReferenceError,TypeError"
class A {
  constructor(x) { this.x = x; }
  m() { return "A.m"; }
}
class B extends A {
  constructor(x, y) { super(x); this.y = y; }
  m() { return super.m() + ">B"; }
}
class C extends B {}
var c = new C(1, 2);
var early;
try { new (class extends A { constructor() { this.x = 0; super(); } })(); } catch (e) { early = e.name; }
var heritage;
try { (class extends 0 {}); } catch (e) { heritage = e.name; }
["B:" + c.x, c.y, c.m(), c instanceof A, Object.getPrototypeOf(C) === B, early, heritage].join();
