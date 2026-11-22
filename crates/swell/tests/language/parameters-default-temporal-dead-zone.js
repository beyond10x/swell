// sec-functiondeclarationinstantiation: throw ReferenceError
// A default that reads a later parameter reads an uninitialized binding.
function f(a = b, b = 1) { return a; }
f();
