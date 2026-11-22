// sec-array.prototype.pop: normal "3|1,2|1|1|0,1,2|5|0,1,2,4,5"
// Then sec-array.prototype.shift and sec-array.prototype.unshift on the same Array.
var a = [1, 2, 3];
var r = [];
r.push(a.pop());
r.push(a.join());
r.push(a.shift());
r.push(a.length);
a.unshift(0, 1);
r.push(a.join());
r.push(a.push(4, 5));
r.push(a.join());
r.join("|");
