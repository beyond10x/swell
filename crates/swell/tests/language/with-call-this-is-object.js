// sec-evaluatecall: normal true
// sloppy-only
var o = { f: function () { return this; } };
with (o) { f() === o; }
