// sec-object.getownpropertydescriptors: normal "1,true,true,true|function,undefined,false,true|true|none,TypeError"
// FromPropertyDescriptor (sec-frompropertydescriptor) of every own key, a data and an accessor
// property, on an object of %Object.prototype%. The Symbol key of Array, @@species, is described
// too: its descriptor object is no constructor, so ArraySpeciesCreate of map throws when it
// reads it (sec-arrayspeciescreate), and a script needs no Symbol value to see it.
var o = { a: 1 };
Object.defineProperty(o, "b", { get: function () { return 2; }, configurable: true });
var d = Object.getOwnPropertyDescriptors(o);
function speciesOf(c) {
  var a = [1];
  a.constructor = c;
  try {
    a.map(function (x) { return x; });
    return "none";
  } catch (e) {
    return e.name;
  }
}
[
  [d.a.value, d.a.writable, d.a.enumerable, d.a.configurable].join(),
  [typeof d.b.get, typeof d.b.set, d.b.enumerable, d.b.configurable].join(),
  Object.getPrototypeOf(d) === Object.prototype,
  speciesOf({}) + "," + speciesOf(Object.getOwnPropertyDescriptors(Array))
].join("|");
