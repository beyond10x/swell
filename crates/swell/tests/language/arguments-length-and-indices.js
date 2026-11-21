// sec-arguments-exotic-objects: normal "3 a b c undefined"
function f(x) { return arguments.length + " " + arguments[0] + " " + arguments[1] + " " + arguments[2] + " " + arguments[3]; }
f("a", "b", "c");
