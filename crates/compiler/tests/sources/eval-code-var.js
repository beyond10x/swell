// sec-evaldeclarationinstantiation: eval code lists its var and function names for EvalDeclarationInstantiation, looks up by name every name it does not bind in a function of its own, and keeps its completion value
var x = 1;
function h(p) { return p + x; }
x + h(2);
