// sec-functiondeclarationinstantiation: with a parameter expression the body has a var scope of its own, a var of a parameter name starting with its value
function f(a, b = function () { return a; }) { var a; var c; return b() + a + c; }
