// sec-updateempty: normal "ok"
// A break or continue out of an if statement, a switch statement or a loop leaves it with
// UpdateEmpty(C, undefined): the value of its body, or undefined. A block or a labelled
// statement that is no loop passes the completion on unchanged, so the statement list around it
// keeps its value. Each source is run through eval with the exit between its halves, once as it
// is and once with "42; " before the exit.
var bad = [];
function check(source, expected) {
  var found;
  try { found = eval(source); } catch (e) { found = "throw " + e; }
  if (found !== expected) bad.push(source + " => " + found);
}
var updated = [
  ["99; do { -99; if (true) { ", " } -77 } while (false);", "break;"],
  ["99; do { -99; if (false) { } else { ", " } -77 } while (false);", "continue;"],
  ["99; do { -99; switch (1) { case 1: ", " } -77 } while (false);", "continue;"],
  ["99; L: { -99; switch (1) { case 0: default: ", " } -77 }", "break L;"],
  ["99; L: { -99; do { ", " } while (false); -77 }", "break L;"],
  ["99; L: { -99; while (true) { ", " } -77 }", "break L;"],
  ["99; L: { -99; for (;;) { ", " } -77 }", "break L;"],
  ["99; L: { -99; for (var k in { a: 1 }) { ", " } -77 }", "break L;"],
  ["99; L: do { -99; while (true) { ", " } -77 } while (false);", "continue L;"],
  ["99; L: do { -99; for (var k in { a: 1 }) { ", " } -77 } while (false);", "continue L;"]
];
var kept = [
  ["99; do { -99; { ", " } -77 } while (false);", "break;"],
  ["99; do { -99; M: { ", " } -77 } while (false);", "continue;"],
  ["99; L: { -99; M: { ", " } -77 }", "break L;"]
];
for (var u = 0; u < updated.length; u++) {
  check(updated[u][0] + updated[u][2] + updated[u][1], undefined);
  check(updated[u][0] + "42; " + updated[u][2] + updated[u][1], 42);
}
for (var k = 0; k < kept.length; k++) {
  check(kept[k][0] + kept[k][2] + kept[k][1], -99);
  check(kept[k][0] + "42; " + kept[k][2] + kept[k][1], 42);
}
bad.length === 0 ? "ok" : bad.join(" | ");
