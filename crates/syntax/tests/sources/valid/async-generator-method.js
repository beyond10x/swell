// sec-async-generator-function-definitions: parses
x = {
  async *g(a, b = 0, ...c) { var d = yield a; await d; yield await b; return c; },
  async *["k" + 1]() {},
  async *"s"() {},
  async *1() {},
  async *get() {},
  async *set(v) {},
  async *async() {},
  async *await() {},
  async *yield() {},
  async *if() {},
  async *
  newline() {},
  async *e({ a }, [b]) { var a; var b; for await (var x of a) yield x; },
  async *f() { var h = async () => await 1; return { async *n() { yield await h(); } }; },
  async *p(q = function* () { yield; }, r = async function () { await 1; }) {},
  m() {},
  *s2() {},
  async m2() {},
  async: 1,
  async,
};
async function outer() {
  return { async *[await 1]() { yield 2; } };
}
function* gen() {
  return { async *[yield 1]() { yield await 2; } };
}
