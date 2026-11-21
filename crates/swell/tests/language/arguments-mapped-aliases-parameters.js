// sec-arguments-exotic-objects: normal "2 3 1 true 1"
// sloppy-only
function f(a) { arguments[0] = 2; var first = a; a = 3; return first + " " + arguments[0] + " " + arguments.length + " " + (arguments.callee === f); }
function g(a) { delete arguments[0]; arguments[0] = 5; return a; }
f(1) + " " + g(1);
