// sec-array.prototype.with: normal "1,9,3|3,2,1|1,a,b,3|1,2,3"
// Then sec-array.prototype.toreversed and sec-array.prototype.tospliced: each answers a new Array and leaves its receiver as it was.
var a = [1, 2, 3];
[a.with(1, 9).join(), a.toReversed().join(), a.toSpliced(1, 1, "a", "b").join(), a.join()].join("|");
