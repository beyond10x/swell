// sec-array.prototype.reverse: normal "4,3,2,1|4,0,0,1|0,0,0,1"
// Then sec-array.prototype.fill from 1 to the last index and sec-array.prototype.copywithin of
// index 1 to index 0, on the same Array.
var a = [1, 2, 3, 4];
var r = [];
r.push(a.reverse().join());
r.push(a.fill(0, 1, -1).join());
r.push(a.copyWithin(0, 1, 2).join());
r.join("|");
