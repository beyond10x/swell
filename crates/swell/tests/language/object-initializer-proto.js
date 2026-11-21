// sec-__proto__-property-names-in-object-initializers: normal 1
var p = { a: 1 };
var o = { __proto__: p };
o.a;
