// sec-functiondeclarationinstantiation: a direct eval in a parameter expression of non-strict code gets a record of its own for its var names
function f(a = eval("var x = 1"), b = x) { var y; return b; }
