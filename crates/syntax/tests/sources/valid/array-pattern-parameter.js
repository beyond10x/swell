// sec-function-definitions: parses
function f([a, b], [c] = [], ...[d, e]) {}
var g = function ([h, [i]] = o) {};
var j = ([k, l = k]) => k;
var m = ([n], { p: [q] }, ...[r]) => n;
var s = { t([u]) {}, set v([w]) {} };
var x = (...[y = 1]) => y;
