// sec-object-environment-records-hasbinding-n: normal 2
// sloppy-only
function f() { var x = 1; var o = { x: 2 }; with (o) { return x; } }
f();
