// sec-runtime-semantics-catchclauseevaluation: normal 2
var r = 0;
try { throw 1; } catch { r = 2; }
r;
