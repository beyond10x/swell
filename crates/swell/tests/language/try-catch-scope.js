// sec-runtime-semantics-catchclauseevaluation: normal "outer"
var e = "outer";
try { throw "inner"; } catch (e) { }
e;
