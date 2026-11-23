// sec-generator-function-definitions-runtime-semantics-evaluation: normal "1,2,a,b,3;sent,9;x,y,true"
// yield* delegates to the iterator of its operand: an array, a string and a generator give their
// values in order, the value given to next reaches the generator delegated to, and the value that
// generator returns is the value of the yield*; a return given while it delegates runs the
// finally of the generator delegated to and completes both with the value given.
var log = [];
function* inner() { var got = yield 3; log.push(got); return 9; }
function* outer() {
  yield* [1, 2];
  yield* "ab";
  var r = yield* inner();
  log.push(r);
}
var g = outer(), values = [];
for (var s = g.next(); !s.done; s = g.next("sent")) values.push(s.value);
var closed = 0;
function* fin() { try { yield "x"; yield "y"; } finally { closed++; } }
function* wrap() { yield* fin(); }
var w = wrap(), xs = [w.next().value, w.next().value], ret = w.return(5);
var ended = closed === 1 && ret.value === 5 && ret.done && w.next().done;
[values.join(), log.join(), xs.join() + "," + ended].join(";");
