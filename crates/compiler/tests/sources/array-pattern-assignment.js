// sec-runtime-semantics-iteratordestructuringassignmentevaluation: each target reference is evaluated before its step, the value of the assignment is its right side, a nested object pattern reads the stepped value
function f(o, p) { return [p.a, { b: p.b }, ...p.c] = o; }
