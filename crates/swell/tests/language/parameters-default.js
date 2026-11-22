// sec-functiondeclarationinstantiation: normal true
// A default is evaluated for a missing or undefined argument only, and may read an earlier one.
function f(a, b = a * 2, c = b + 1) { return [a, b, c].join(); }
var o = { m(x = "m") { return x; }, set s(v = "set") { this.seen = v; } };
o.s = undefined;
f(1) === "1,2,3" && f(1, undefined, 0) === "1,2,0" && f(1, null) === "1,,1"
  && o.m() === "m" && o.m("given") === "given" && o.seen === "set";
