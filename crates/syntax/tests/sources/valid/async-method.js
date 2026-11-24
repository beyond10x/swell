// sec-async-function-definitions: parses
x = {
  async m(a, b = 0, ...c) { await a; return await b; },
  async ["k" + 1]() {},
  async "s"() {},
  async 1() {},
  async get() {},
  async set(v) {},
  async async() {},
  async await() {},
  async if() {},
  async d(a = 1, { b } = {}, [c] = []) { var e = await (a + b + c); return e; },
  async e() { var f = async () => await 1; return { async n() { await f(); } }; },
  m() {},
  *g() {},
  get async() { return 1; },
  async: 1,
  async,
};
async function outer() {
  return { async [await 1]() { await 2; } };
}
function* gen() {
  return { async [yield 1]() { await 2; } };
}
