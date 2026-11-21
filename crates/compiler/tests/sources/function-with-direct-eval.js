// sec-functiondeclarationinstantiation: a non-strict function with a direct eval keeps every binding in its named function environment, makes its arguments object, and looks up by name what it does not bind; a strict one keeps its slots and the global names
var g = 1;
function f(a) { var b = a; eval("b"); function inner() { return a + b + g; } return b + g; }
function s(a) { "use strict"; var b = a; eval("b"); return b + g; }
