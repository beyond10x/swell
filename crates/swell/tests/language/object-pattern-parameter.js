// sec-functiondeclarationinstantiation: normal true
// A pattern as a parameter of a function, a method, a setter and an arrow makes the list not
// simple: the names bind in order, a default may read an earlier name, length counts a pattern
// without an initializer and the arguments object is unmapped.
function f(x, { a, b = a + 1 }, y) {
  arguments[0] = 9;
  return [x, a, b, y].join();
}
var o = {
  m({ p }) { return p; },
  set s({ q }) { this.seen = q; }
};
o.s = { q: 3 };
var arrow = ({ v }, w = v) => v + w;
var thrown = "none";
try { f(1); } catch (e) { thrown = e.name; }
f(1, { a: 2 }, 3) === "1,2,3,3" && f.length === 3 && o.m({ p: 7 }) === 7
  && o.seen === 3 && arrow({ v: 2 }) === 4 && arrow.length === 1 && thrown === "TypeError";
