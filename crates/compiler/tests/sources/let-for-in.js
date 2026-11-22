// sec-runtime-semantics-forinofheadevaluation: the expression of a for-in head sees the head's names uninitialized, and a binding a closure captures gets a record of its own each iteration
for (let k in { a: k }) { k; }
var f;
for (const k in { a: 1 }) { f = function () { return k; }; }
