// sec-generator-function-definitions-runtime-semantics-evaluation: a generator declaration starts after FunctionDeclarationInstantiation, suspends at each yield and dispatches on the tag of its resumption
function* g(a) {
  var b = yield a;
  yield;
  return b;
}
