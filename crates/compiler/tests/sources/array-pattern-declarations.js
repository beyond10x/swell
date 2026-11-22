// sec-runtime-semantics-iteratorbindinginitialization: a var pattern holds the iterator on the stack under a pattern handler, steps once per element and elision, runs a default for undefined, collects a rest, and closes the iterator at its end
function f(o) { var [a, , b = 1] = o; let [[c], ...d] = o; return a + b + c + d; }
