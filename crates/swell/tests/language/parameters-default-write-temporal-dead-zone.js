// sec-declarative-environment-records-setmutablebinding-n-v-s: throw ReferenceError
// A direct eval in a default that writes a later parameter finds it uninitialized.
function f(a = eval("b = 1"), b) { return a; }
f();
