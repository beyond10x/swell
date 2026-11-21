// sec-evaldeclarationinstantiation: normal "inner true true outer"
// sloppy-only
var x = "outer";
function f() {
  eval("var x = 'inner'; function g() { return x; }");
  var seen = x + " " + (g() === "inner");
  var deleted = delete x;
  return seen + " " + deleted + " " + x;
}
f();
