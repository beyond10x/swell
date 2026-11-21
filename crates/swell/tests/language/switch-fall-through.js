// sec-runtime-semantics-caseblockevaluation: normal "bcd"
var s = "";
switch (2) { case 1: s += "a"; case 2: s += "b"; default: s += "c"; case 4: s += "d"; }
s;
