// sec-runtime-semantics-definemethod: throw TypeError
var o = { m() {} };
if (o.m.hasOwnProperty("prototype")) throw "a method has a prototype";
new o.m();
