// sec-template-literals-runtime-semantics-evaluation: normal true
// Each substitution goes through ToString: primitives, an array, an object with toString, a
// function's result; the strings between them are kept in order.
var n = 1.5;
var a = [1, 2];
var o = { toString: function () { return "O"; } };
function f() { return "F"; }
var all = `${undefined}|${null}|${true}|${n}|${a}|${o}|${f()}|${"s"}`;
var spans = `a${1}b${2}c${3}d`;
var edges = `${1}${2}`;
var inside = `${"x" in { x: 1 }}`;
all === "undefined|null|true|1.5|1,2|O|F|s" && spans === "a1b2c3d" && edges === "12" && inside === "true";
