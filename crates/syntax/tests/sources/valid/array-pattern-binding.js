// sec-destructuring-binding-patterns: parses
var [a, , b = 1, [c, d = 2] = [], { e }, ...f] = o;
let [g, [h]] = o, [i] = o;
const [j = function () {}, k = () => 0] = o;
var [] = o, [,] = o, [, , l] = o;
for (var [m] = o; m; ) {}
for (let [n, p] = o; n; ) {}
for (var [q] in o) {}
for (let [r, s] in o) {}
for (const [t, u] of o) {}
for (let [v] of o) {}
try {} catch ([w, x = w]) {}
var [...[y, ...z]] = o;
