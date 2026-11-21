// sec-resolvebinding: normal 3
var g = 1;
function f() { var g = 2; return function () { return g; }; }
f()() + g;
