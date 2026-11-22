// sec-runtime-semantics-iteratorbindinginitialization: a rest parameter is an Array of the arguments after the parameters before it, and the arguments object is unmapped
function f(a, ...r) { return arguments.length + r.length; }
