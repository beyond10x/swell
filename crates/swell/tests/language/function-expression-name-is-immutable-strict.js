// sec-declarative-environment-records-setmutablebinding-n-v-s: throw TypeError
// strict-only
var f = function g() { g = 1; return typeof g; };
f();
