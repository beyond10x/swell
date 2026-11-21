// sec-functiondeclarationinstantiation: normal "true,3"
function f(a, b) { return b === undefined; }
function g() { return h(); function h() { return 3; } }
f(1) + "," + g();
