// sec-generator-function-definitions-runtime-semantics-evaluation: parses
var before = async function* () { yield 1; };
async function* f(a, b) { function* g() { yield* a; }
  yield* b;
  yield *
    b;
  x = yield* [1, 2];
  yield* yield* 0;
  f(yield* a, yield *b);
  await (yield* a);
}
