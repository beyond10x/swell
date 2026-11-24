// sec-async-generator-function-definitions: refused "async generator function is not supported before slice 10"
var before = async function () { await 0; };
async function* declared(a, b = 1, { c }, [d], ...e) {
  yield;
  yield a;
  await b;
  yield await c;
  var x = await (yield d);
  var y = { *m() { yield 1; }, async n() { await 2; } };
  var z = async (p) => await p;
  async function* inner() {
    yield inner;
  }
  var inner;
  return e;
}
var expression = async function* () {}, named = async function* named(f) {
  "use strict";
  yield f;
};
async function* yield() {}
async function* await() {}
{
  async function* inBlock() {}
}
switch (0) {
  case 0:
    async function* inCase() {}
}
try {
  async function* inTry() {}
} catch (e) {
  async function* inCatch() {}
} finally {
  async function* inFinally() {}
}
function outer() {
  return async function* (yield_, await_) {
    for (var i of [yield_, await_]) yield i;
  };
}
