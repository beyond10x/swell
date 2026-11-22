// sec-runtime-semantics-iteratorbindinginitialization: a pattern parameter, a pattern with a default and a rest parameter that is a pattern
function f([a], [b] = [], ...[c]) { return a + b + c; }
