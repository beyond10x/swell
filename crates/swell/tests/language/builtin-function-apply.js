// sec-function.prototype.apply: normal "t|2|1|2|undefined;t|3|7|8|9;t|2|x|y|undefined;t|2|undefined|5|undefined;t|0|undefined|undefined|undefined;t|0|undefined|undefined|undefined;TypeError;TypeError"
// Then sec-createlistfromarraylike: an array, an arguments object, an object with "length"
// and an array with a hole give the arguments element by element; undefined and null give
// none; a primitive argument list and a this value that is not callable throw a TypeError.
var t = { k: "t" };
function f(a, b, c) {
  return this.k + "|" + arguments.length + "|" + a + "|" + b + "|" + c;
}
var r = f.apply(t, [1, 2]);
r += ";" + (function () { return f.apply(t, arguments); })(7, 8, 9);
r += ";" + f.apply(t, { length: 2, 0: "x", 1: "y", 2: "z" });
r += ";" + f.apply(t, [, 5]);
r += ";" + f.apply(t, undefined);
r += ";" + f.apply(t, null);
try { f.apply(t, 1); } catch (e) { r += ";" + e.name; }
try { Function.prototype.apply.call({}, t, []); } catch (e) { r += ";" + e.name; }
r;
