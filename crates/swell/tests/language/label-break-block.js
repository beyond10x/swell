// sec-labelled-statements-runtime-semantics-labelledevaluation: normal 1
var x = 0;
a: { x = 1; break a; x = 2; }
x;
