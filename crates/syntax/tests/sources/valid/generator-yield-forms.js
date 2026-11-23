// sec-generator-function-definitions: parses
// sloppy-only
function* g(a, b = 0, ...c) {
  yield;
  yield 1;
  yield
  1;
  x = yield;
  x = yield a, b;
  var y = yield, z = yield 2;
  let w = yield;
  f(yield, yield 2, ...yield);
  [yield, , yield 1, ...yield];
  ({ a: yield, [yield]: 0, ...yield, b: yield 1 });
  (yield);
  (yield 1, yield);
  yield yield yield 0;
  yield /re/g;
  yield `t${yield}`;
  yield {};
  yield [];
  yield (0);
  yield -1, yield +1, yield !a, yield ~a;
  yield ++a;
  yield typeof a, yield void a, yield delete a.b;
  yield new X();
  yield this;
  yield null, yield true, yield "s", yield 1.5;
  yield function () {};
  yield function* () { yield; };
  yield a => a;
  yield (a) => yield;
  x = a ? yield : yield 1;
  if (yield) yield; else yield 1;
  while (yield) yield;
  for (yield; yield; yield) yield;
  switch (yield) { case yield: yield; }
  return yield;
  function h() {
    var yield = 1;
    return yield * yield;
  }
  (function yield() {});
  () => yield;
  yield
}
