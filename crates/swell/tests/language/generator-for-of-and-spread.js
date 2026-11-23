// sec-generator-objects: normal "1,2,3;1,2,3;6;a,b;closed;true;3"
// A generator object is an iterator whose Symbol.iterator answers itself (inherited from
// %Iterator.prototype%): a for-of, a spread and an array pattern step it, and an array pattern
// that stops before the end, or a break, calls its return, which runs its finally block.
function* three() { yield 1; yield 2; yield 3; }
var seen = [];
for (var v of three()) seen.push(v);
var spread = [...three()];
var total = 0;
for (const x of three()) total += x;
var log = [];
function* letters() { try { yield "a"; yield "b"; yield "c"; } finally { log.push("closed"); } }
var [p, q] = letters();
var it = three();
seen.join() + ";" + spread.join() + ";" + total + ";" + p + "," + q + ";" + log.join() + ";" +
  (it[Symbol.iterator]() === it) + ";" + Math.max(...three());
