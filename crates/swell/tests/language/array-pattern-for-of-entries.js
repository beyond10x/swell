// sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset: normal "a=1,b=2;3;a,b;x1"
// A for-of over Object.entries binds each entry's key and value through an array pattern in its
// head, a let pattern gives each iteration bindings of its own, and an assignment pattern in
// the head assigns existing bindings.
var pairs = [], sum = 0, keys = [], fs = [];
for (const [k, v] of Object.entries({ a: 1, b: 2 })) pairs.push(k + "=" + v);
for (var [, v] of [["x", 1], ["y", 2]]) sum += v;
for (let [k] of [["a"], ["b"]]) fs.push(function () { return k; });
var name, value;
for ([name, value] of [["x", 1]]) ;
pairs.join() + ";" + sum + ";" + fs.map(function (f) { return f(); }).join() + ";" + name + value;
