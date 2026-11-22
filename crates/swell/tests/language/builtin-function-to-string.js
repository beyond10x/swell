// sec-function.prototype.tostring: normal "true,true,true,true,true,true,true,TypeError"
// Then the source text of a declaration, an expression, a getter and a plain method, comments
// and line ends inside kept and the text around left out; a NativeFunction string
// (prod-NativeFunction) for a built-in and a bound function; a TypeError for an object that
// is not callable.
/* before */ function d(a, /* in */ b) {
  return a;
} /* after */
var e = /* lead */ function (x) { return x; } /* tail */;
var o = { get p() { return 1; }, m() {} };
var r = "" + (d.toString() === "function d(a, /* in */ b) {\n  return a;\n}");
r += "," + (e.toString() === "function (x) { return x; }");
r += "," + (Object.getOwnPropertyDescriptor(o, "p").get.toString() === "get p() { return 1; }");
r += "," + (o.m.toString() === "m() {}");
r += "," + (Function.prototype.call.toString() === "function call() { [native code] }");
r += "," + (d.bind(null).toString() === "function () { [native code] }");
r += "," + (Function.prototype.toString.call(Function.prototype) === "function () { [native code] }");
try { Function.prototype.toString.call({}); } catch (x) { r += "," + x.name; }
r;
