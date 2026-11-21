// sec-array.prototype.map: normal "2,4,6"
// Then sec-array.prototype.join with no separator: ",".
[1, 2, 3].map(function (x) { return x * 2; }).join();
