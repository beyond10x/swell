// sec-runtime-semantics-instantiategeneratorfunctionexpression: a named generator expression, its own name bound, and a return inside a yield expression
var e = function* named(x) {
  return named === x ? yield 1 : 2;
};
