// sec-labelled-statements-runtime-semantics-labelledevaluation: normal "tfe,tgfe"
var log = "";
a: try { log += "t"; break a; } finally { log += "f"; }
log += "e,";
b: while (true) { try { log += "t"; while (true) { try { break b; } finally { log += "g"; } } } finally { log += "f"; } }
log += "e";
log;
