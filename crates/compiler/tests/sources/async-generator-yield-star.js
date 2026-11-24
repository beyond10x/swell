// sec-generator-function-definitions-runtime-semantics-evaluation: in an async generator yield* delegates to the async iterator of its operand in a loop of the delegating frame that awaits each inner result, yields its value without a second await, awaits a received return before it passes it on, and awaits the value of a return the iterator has no method for; a throw drops the record
async function* f(g) {
  var x = yield* g;
}
