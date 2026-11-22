// sec-runtime-semantics-destructuringassignmentevaluation: normal true
// An object literal on the left of = is read again as an assignment pattern: its targets are
// names, members and indices, its value is the right-hand side, and for-in assigns each key
// through one.
var a, b, o = {}, list = [];
var result = ({ a, x: b, y: o.member, z: list[0], w: a = 5 } = { a: 1, x: 2, y: 3, z: 4 });
var keys = [];
var holder = {};
for ({ length: holder.n } in { ab: 0, cde: 0 }) { keys.push(holder.n); }
var chained, again;
chained = { k: again } = { k: 6 };
a === 5 && b === 2 && o.member === 3 && list[0] === 4 && result.z === 4
  && keys.join() === "2,3" && chained.k === 6;
