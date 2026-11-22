// sec-static-semantics-expectedargumentcount: normal true
// The length of a function counts its parameters before the first default or rest parameter.
function a(x, y, z = 0, w) {}
function b(...r) {}
var c = function (x, ...r) {};
var d = (x = 0, y) => 0;
a.length === 2 && b.length === 0 && c.length === 1 && d.length === 0;
