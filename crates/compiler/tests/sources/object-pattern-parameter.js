// sec-functiondeclarationinstantiation: a parameter pattern makes the list not simple, its names are slots the code binds in order after the argument and its default
function f({ a, b = a } = {}, c) { return a + b + c; }
