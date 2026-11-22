// sec-array.prototype.filter: normal "true|2,4,6|b,d"
// filter keeps the values its callback selects, in order, numbered from 0, in a new Array
// (sec-arrayspeciescreate); holes are not visited; an array-like works.
var even = [1, 2, , 4, 5, 6].filter(function (v) { return v % 2 === 0; });
var picked = Array.prototype.filter.call({ length: 4, 1: "b", 3: "d" }, function () { return true; });
[Array.isArray(even), even.join(), picked.join()].join("|");
