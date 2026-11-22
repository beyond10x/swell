// sec-object.assign: normal "true|a=1,b=2|0,1|TypeError:1:0:undefined"
// Sources undefined and null are skipped, a String source gives its indices, and the first Set
// that fails (a property that is not writable) throws a TypeError and keeps what was copied.
var target = { a: 0 };
var result = Object.assign(target, { a: 1 }, null, undefined, { b: 2 });
var fixed = {};
Object.defineProperty(fixed, "b", { value: 0, writable: false, enumerable: true, configurable: true });
var outcome;
try {
  Object.assign(fixed, { a: 1, b: 2, c: 3 });
  outcome = "no throw";
} catch (err) {
  outcome = err.name + ":" + fixed.a + ":" + fixed.b + ":" + fixed.c;
}
[
  result === target,
  "a=" + target.a + ",b=" + target.b,
  Object.keys(Object.assign({}, "ab")).join(),
  outcome
].join("|");
