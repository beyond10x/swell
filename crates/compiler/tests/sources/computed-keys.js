// sec-runtime-semantics-propertydefinitionevaluation: a computed key is evaluated and converted before its value, an anonymous function value, a method and an accessor are named at run time
var k, m, g, s;
x = { a: 1, [k]: 2, [k + 1]: function () {}, [m]() {}, get [g]() { return 0; }, set [s](v) {} };
