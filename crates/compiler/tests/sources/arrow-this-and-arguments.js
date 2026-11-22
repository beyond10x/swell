// sec-arrow-function-definitions: this in an arrow is the This of its own frame, and its arguments is the binding of the enclosing function, a slot of that function's environment
function f(a) { "use strict"; return () => this === arguments[0]; }
