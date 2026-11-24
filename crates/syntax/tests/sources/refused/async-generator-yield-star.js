// sec-generator-function-definitions-runtime-semantics-evaluation: refused "yield* in an async generator is not supported before slice 10"
var before = async function* () { yield 1; };
async function* f(a, b) { function* g() { yield* a; }
  yield* b;
}
