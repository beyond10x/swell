// sec-functiondeclarationinstantiation: the code binds each parameter from the kept arguments, a default only for undefined, and length counts the parameters before the first default
function f(a, b = a) { return b; }
