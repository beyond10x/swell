// sec-performeval: a direct eval in an arrow gives the nearest enclosing function that is not an arrow its arguments object, a named slot of its environment, and the arrow an environment of its own for the var names of the eval code
function f() { return () => eval("var x = arguments"); }
