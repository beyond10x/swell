// sec-async-function-definitions: refused "async function is not supported before slice 10"
async function f(a, b = 1, ...c) {
  await a;
  await await b;
  var d = await (a + b), e = -await a, g = !await a;
  typeof await a, void await a, delete await a.b;
  await /re/g;
  await `t${await a}`;
  return await a ? await b : await c;
}
var h = async function () {
  await 1;
};
var i = async function named(x) {
  var y = await x;
  return y;
};
var j = async x => await x;
var k = async (x, y) => {
  await x;
};
var l = async () => 1;
var m = async ({ a }, [b], c = 2, ...d) => a;
var n = async (...[e]) => e;
var o = async (a, b,) => a + b;
var p = async ({ a = 1 }) => a;
{
  async function inner() {}
}
switch (0) {
  case 0:
    async function inCase() {}
}
async function outer() {
  function plain() {
    var await;
    return await;
  }
  function* gen() {
    var await = yield;
  }
  async function nested() {
    await nested;
  }
  var arrow = async () => await outer;
  var plainArrow = (a = 1) => a;
  for (var x of await a);
  if (await a) await b;
}
