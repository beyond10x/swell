// sec-runtime-semantics-iteratorbindinginitialization: normal true
// A rest parameter is a new Array of the arguments after the parameters before it.
function f(a, b, ...rest) { return rest; }
var some = f(1, 2, 3, 4);
var none = f(1);
Array.isArray(some) && some.join() === "3,4" && Array.isArray(none) && none.length === 0
  && f.length === 2;
