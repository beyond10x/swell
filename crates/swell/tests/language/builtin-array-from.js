// sec-array.from: normal "abc|0,2,4|x,y|3|true"
// Array.from reads a String by code point and an iterable the script writes through its
// iterator, maps each value with its index, and reads an array-like object by its length.
var r = [];
r.push(Array.from("abc").join(""));
r.push(Array.from([0, 1, 2], function (v, k) { return v + k; }).join());
r.push(Array.from({ length: 2, 0: "x", 1: "y" }).join());
var o = {};
o[Symbol.iterator] = function () {
  var i = 0;
  return { next: function () { i++; return { value: i, done: i > 3 }; } };
};
r.push(Array.from(o).length);
r.push(Array.isArray(Array.from(o)));
r.join("|");
