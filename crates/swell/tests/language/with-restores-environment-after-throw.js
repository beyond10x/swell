// sec-with-statement-runtime-semantics-evaluation: normal "outer"
// sloppy-only
var x = "outer";
var o = { x: "inner" };
function f() {
  try { with (o) { throw 0; } } catch (e) {}
  with ({}) { return x; }
}
f();
