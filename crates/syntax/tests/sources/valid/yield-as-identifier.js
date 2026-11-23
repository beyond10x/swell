// sec-identifiers-static-semantics-early-errors: parses
// sloppy-only
var yield = 1;
yield: for (;;) break yield;
function f(yield) {
  return yield + 1;
}
yield = yield * 2;
yield++;
var o = { yield };
var a = (yield) => yield;
function yield2() {
  var yield;
}
(function yield() {});
