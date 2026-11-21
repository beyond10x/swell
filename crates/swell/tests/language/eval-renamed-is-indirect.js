// sec-function-calls-runtime-semantics-evaluation: normal "global local"
var x = "global";
function f() {
  var x = "local";
  var renamed = eval;
  return renamed("x") + " " + eval("x");
}
f();
