// sec-runtime-semantics-caseblockevaluation: normal "db"
var s = "";
switch (5) { case 1: s += "a"; default: s += "d"; case 2: s += "b"; }
s;
