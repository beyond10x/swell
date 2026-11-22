// sec-array.of: normal "3,7,8,9,true"
// The items become the elements of a new Array, even a single Number, which the Array constructor would take as a length.
var a = Array.of(7, 8, 9);
var b = Array.of(3);
[a.length, a.join(), b.length === 1 && b[0] === 3].join();
