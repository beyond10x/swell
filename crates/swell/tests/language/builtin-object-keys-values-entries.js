// sec-object.keys: normal "0,1|0,2,x|a,c|1,3|a=1;c=3"
// Then sec-object.values and sec-object.entries: EnumerableOwnProperties
// (sec-enumerableownproperties) of a String, an Array with a hole and an object with a property
// that is not enumerable, in the order of [[OwnPropertyKeys]]; a result is compared through join.
var a = [1, , 3];
a.x = 4;
var o = { a: 1, b: 2, c: 3 };
Object.defineProperty(o, "b", { enumerable: false });
var e = Object.entries(o);
[
  Object.keys("ab").join(),
  Object.keys(a).join(),
  Object.keys(o).join(),
  Object.values(o).join(),
  e[0].join("=") + ";" + e[1].join("=")
].join("|");
