// sec-functiondeclarationinstantiation: normal 7
// sloppy-only
// A parameter named arguments (an early error in strict code) is the binding; no object is made.
function f(arguments) { return arguments; }
f(7);
