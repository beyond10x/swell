// sec-generator-function-definitions: parses
x = {
  *g(a, b = 0, ...c) { yield a; yield* c; return b; },
  *["k" + 1]() {},
  *"s"() {},
  *1() {},
  *get() {},
  *set(v) {},
  *async() {},
  *yield() { yield; },
  *if() {},
  *
  newline() {},
  *e({ a }, [b]) { var a; var b; },
  m() {},
};
function* outer() {
  return { *[yield 1]() { yield 2; }, *h(p = function* () { yield; }) {} };
}
