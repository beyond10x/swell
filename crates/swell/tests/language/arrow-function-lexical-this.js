// sec-arrow-function-definitions: normal true
// this in an arrow is the this of the code around it, whatever the arrow is called with.
var o = { m: function () { return () => this; } };
var f = o.m();
var other = {};
var top = (() => this)();
function g() { return (() => () => this)()(); }
var viaEval = (function () { return () => eval("this"); }).call(other)();
f() === o && f.call(other) === o && f.apply(other, []) === o && f.bind(other)() === o
  && g.call(other) === other && top === this && viaEval === other;
