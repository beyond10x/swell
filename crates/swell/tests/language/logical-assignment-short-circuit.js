// sec-assignment-operators-runtime-semantics-evaluation: normal "ggs3g:5:6:7"
var log = "";
var o = { get p() { log += "g"; return 1; }, set p(v) { log += "s" + v; } };
o.p ||= 2;
o.p &&= 3;
o.p ??= 4;
var u;
u ??= 5;
var z = 0;
z ||= 6;
var w = 7;
w ??= (log += "x", 8);
log + ":" + u + ":" + z + ":" + w;
