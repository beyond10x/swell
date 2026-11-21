// sec-declarative-environment-records-setmutablebinding-n-v-s: normal "function"
// sloppy-only
var f = function g() { g = 1; return typeof g; };
f();
