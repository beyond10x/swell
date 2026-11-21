// sec-eval-x: normal "global global global"
var x = "global";
function f() {
  var x = "local";
  var e = eval;
  e("var z = x");
  return e("x") + " " + (0, eval)("x") + " " + z;
}
f();
