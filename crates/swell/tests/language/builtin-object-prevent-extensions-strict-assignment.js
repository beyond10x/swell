// sec-object.preventextensions: normal "2falsetrueundefined"
// strict-only
// An assignment of a new property to an object that is not extensible throws a TypeError in strict code (sec-putvalue); an existing property is still written.
var o = Object.preventExtensions({ a: 1 });
o.a = 2;
var r = "" + o.a + Object.isExtensible(o);
try { o.x = 1; } catch (e) { r += e instanceof TypeError; }
r + o.x;
