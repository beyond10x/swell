// sec-runtime-semantics-forinofheadevaluation: the expression of a for-of head sees the head's names uninitialized, and a binding a closure captures gets a record of its own each iteration
for (let x of [x]) { x; }
var f;
for (const y of a) { f = function () { return y; }; }
