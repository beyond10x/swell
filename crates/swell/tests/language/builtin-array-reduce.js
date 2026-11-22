// sec-array.prototype.reduce: normal "10|scba|TypeError"
// reduce folds ascending from the first element, reduceRight (sec-array.prototype.reduceright)
// descending from the initial value; no element and no initial value throws a TypeError.
var sum = [1, 2, 3, 4].reduce(function (a, v) { return a + v; });
var right = ["a", "b", "c"].reduceRight(function (a, v) { return a + v; }, "s");
var error = "none";
try { [, ,].reduce(function () {}); } catch (e) { error = e.name; }
[sum, right, error].join("|");
