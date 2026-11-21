// sec-variable-statement-runtime-semantics-evaluation: normal "2 undefined"
// sloppy-only
function f() { var o = { x: 1 }; with (o) { var x = 2; } return o.x + " " + x; }
f();
