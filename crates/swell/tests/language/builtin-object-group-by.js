// sec-object.groupby: normal "odd+even|1,3|2|true"
// Object.groupBy puts each value in the group of the key its callback answers
// (sec-groupby), the groups in the order their keys were first seen, each an Array, on an
// object whose prototype is null.
var g = Object.groupBy([1, 2, 3], function (v) { return v % 2 ? "odd" : "even"; });
[Object.keys(g).join("+"), g.odd.join(), g.even.join(), Object.getPrototypeOf(g) === null].join("|");
