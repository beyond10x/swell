// sec-runtime-semantics-instantiateordinaryfunctionexpression: a named function expression sees its own name in an environment of its own
(function fact(n) {
  fact = 0;
  return fact(n);
});
(function g() { var g; return g; });
