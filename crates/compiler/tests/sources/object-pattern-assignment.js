// sec-runtime-semantics-keyeddestructuringassignmentevaluation: each target's reference is evaluated before its property is read, and the assignment's value is the value destructured
var a, o, p;
x = ({ a, b: p.q, c: p[a] = 2 } = o);
