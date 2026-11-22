// sec-arrow-function-definitions: normal true
// arguments in an arrow is the arguments object of the nearest enclosing function that is not an arrow;
// a var of non-strict eval code in an arrow is the arrow's own (strict eval code keeps its own).
function f(a, b) {
  var read = () => arguments[0] + arguments.length;
  var write = () => { arguments[1] = 10; };
  var r = read();
  write();
  var nested = () => () => arguments[1];
  var count = (x) => arguments.length;
  return [r, arguments[1], nested()(), count(1, 2, 3, 4)].join();
}
var viaEval = (function () { return () => eval("arguments.length"); })(1, 2)();
function g() {
  var inner = (() => { eval("var own = 1"); return typeof own; })();
  return inner + "," + typeof own;
}
var strict = (function () { return this === undefined; })();
var scoped = g() === (strict ? "undefined" : "number") + ",undefined" && typeof own === "undefined";
f(1, 2, 3) === "4,10,10,3" && viaEval === 2 && scoped && (() => typeof arguments)() === "undefined";
