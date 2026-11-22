// sec-function-definitions: parses
function f(a, b = a, c = function () { return b; }) {}
var g = function (a = 1,) {};
var h = function named(x = named) {};
var o = { m(a = 1, b) {}, set s(v = 0) {}, get t() { return 0; } };
function k(a = (b = 1) => b, c) { var a; let d = c; }
