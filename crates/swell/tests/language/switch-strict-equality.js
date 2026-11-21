// sec-runtime-semantics-caseblockevaluation: normal "str"
var r;
switch ("1") { case 1: r = "num"; break; case "1": r = "str"; }
r;
