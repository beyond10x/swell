// sec-runtime-semantics-forinofheadevaluation: normal 0
var n = 0;
for (var k in null) n++;
for (var k in undefined) n++;
n;
