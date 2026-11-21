// sec-object-initializer-runtime-semantics-propertydefinitionevaluation: normal "1:true:true"
var a = 1, __proto__ = 2;
var o = { a, __proto__ };
o.a + ":" + o.hasOwnProperty("__proto__") + ":" + (o.__proto__ === 2);
