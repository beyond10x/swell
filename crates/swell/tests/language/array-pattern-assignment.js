// sec-destructuring-assignment: normal "2,1;3;b,c;true;closed"
// An array assignment pattern swaps two bindings, assigns a property and a rest target, its
// value is its right side, and it closes an iterator it did not run to the end.
var x = 1, y = 2, o = {}, rest;
[x, y] = [y, x];
[o.p, ...rest] = ["3", "b", "c"];
var right = [0];
var same = ([o.q] = right) === right;
var log = [];
var it = {};
it[Symbol.iterator] = function () {
  return {
    next: function () { return { value: 1, done: false }; },
    "return": function () { log.push("closed"); return {}; }
  };
};
var w;
[w] = it;
x + "," + y + ";" + o.p + ";" + rest.join() + ";" + same + ";" + log.join();
