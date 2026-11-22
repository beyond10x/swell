// sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset: normal "0,1,2;2,2,2;a,b"
// let in the head of a for-of gives each iteration a binding of its own, which a closure made
// in that iteration keeps; var shares one binding; const too gives each iteration its own.
var lets = [], vars = [], consts = [];
for (let i of [0, 1, 2]) lets.push(function () { return i; });
for (var j of [0, 1, 2]) vars.push(function () { return j; });
for (const k of ["a", "b"]) consts.push(() => k);
function call(f) { return f(); }
lets.map(call).join() + ";" + vars.map(call).join() + ";" + consts.map(call).join();
