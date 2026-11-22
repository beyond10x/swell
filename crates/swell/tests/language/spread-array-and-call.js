// sec-runtime-semantics-arrayaccumulation: normal "0,1,2,3;3;5;1,2;a,b"
// A spread in an array literal, in a call, in new and over a String: each takes the values of
// its iterable in order among the other elements and arguments.
var a = [0, ...[1, 2], 3];
var holes = [, ...[1], ,];
var max = Math.max(...[1, 5], ...[2]);
function Pair(x, y) { this.text = x + "," + y; }
var pair = new Pair(...[1, 2]);
a.join() + ";" + holes.length + ";" + max + ";" + pair.text + ";" + [..."ab"].join();
