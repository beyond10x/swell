// sec-array.prototype.sort: normal "1,10,2,9|1,2,9,10|c,a,b|1,2,9,10,"
// Without a comparator by the code units of the Strings, with one by its answer; equal keys
// keep their order; sec-array.prototype.tosorted reads a hole as undefined and sorts it last.
var a = [10, 9, 2, 1];
var r = [];
r.push(a.sort().join());
r.push(a.sort(function (x, y) { return x - y; }).join());
var s = [{ k: 1, n: "a" }, { k: 1, n: "b" }, { k: 0, n: "c" }];
s.sort(function (x, y) { return x.k - y.k; });
r.push(s[0].n + "," + s[1].n + "," + s[2].n);
var h = [10, , 2, 9, 1];
r.push(h.toSorted(function (x, y) { return x - y; }).join());
r.join("|");
