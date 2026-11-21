// sec-runtime-semantics-instantiateordinaryfunctionexpression: normal 120
var f = function g(n) { return n ? n * g(n - 1) : 1; };
f(5);
