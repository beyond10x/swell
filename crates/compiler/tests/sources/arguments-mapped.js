// sec-createmappedargumentsobject: the parameters of a non-strict function that references arguments live in its function environment, which the mapped object reads
function f(a, b) { return arguments[0] + a; }
