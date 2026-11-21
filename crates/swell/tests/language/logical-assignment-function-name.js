// sec-assignment-operators-runtime-semantics-evaluation: normal "fgh"
var f;
f ||= function () {};
var g = 1;
g &&= function () {};
var h = null;
h ??= function () {};
f.name + g.name + h.name;
