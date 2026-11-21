// sec-runtime-semantics-catchclauseevaluation: normal 7
var f;
try { throw 7; } catch (e) { f = function () { return e; }; }
f();
