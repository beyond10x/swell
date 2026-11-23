// sec-generator-function-definitions: refused "generator function is not supported before slice 7"
var g = function* () {
  yield* h();
  yield *
    h();
  x = yield* [1, 2];
  yield* yield* 0;
  f(yield* a, yield *b);
};
