// sec-with-statement-runtime-semantics-evaluation: normal "ok"
// sloppy-only
// A break or continue out of a with statement leaves it with UpdateEmpty(C, undefined): the
// value of its body, or undefined. Each source is run through eval with the exit between its
// halves, once as it is and once with "42; " before the exit.
var bad = [];
function check(source, expected) {
  var found;
  try { found = eval(source); } catch (e) { found = "throw " + e; }
  if (found !== expected) bad.push(source + " => " + found);
}
var places = [
  ["99; do { -99; with ({}) { ", " } -77 } while (false);", "break;"],
  ["99; do { -99; with ({}) { ", " } -77 } while (false);", "continue;"],
  ["99; L: { -99; with ({}) { ", " } -77 }", "break L;"],
  ["99; do { -99; with ({}) { try { 39 } finally { ", " } } -77 } while (false);", "break;"]
];
for (var p = 0; p < places.length; p++) {
  check(places[p][0] + places[p][2] + places[p][1], undefined);
  check(places[p][0] + "42; " + places[p][2] + places[p][1], 42);
}
bad.length === 0 ? "ok" : bad.join(" | ");
