// sec-generator-function-definitions-runtime-semantics-evaluation: yield* delegates to the iterator of its operand in a loop of the delegating frame, whose record a throw drops, and a return it is done with leaves through the finally around the body
function* f(g) {
  var x = yield* g;
}
