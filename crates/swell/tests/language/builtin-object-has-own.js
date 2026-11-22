// sec-object.hasown: normal "true,false,false,true,TypeError"
// An inherited property is not own; an index of a String is. ToObject comes before
// ToPropertyKey, so the key's toString does not run for a null object.
function C() {}
C.prototype.p = 1;
var log = [];
try {
  Object.hasOwn(null, { toString: function () { log.push("key"); return "a"; } });
} catch (e) {
  log.push(e.name);
}
[
  Object.hasOwn({ a: 1 }, "a"),
  Object.hasOwn({ a: 1 }, "b"),
  Object.hasOwn(new C(), "p"),
  Object.hasOwn("ab", 1),
  log.join()
].join();
