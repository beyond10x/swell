// sec-with-statement-runtime-semantics-evaluation: normal "2 2 1"
// sloppy-only
var x = 1;
var o = { x: 2 };
var r;
with (o) { r = eval("x"); eval("var x = 3"); }
r + " " + (o.x - 1) + " " + x;
