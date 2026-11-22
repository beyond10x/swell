// sec-try-statement-runtime-semantics-evaluation: normal "ok"
// A finally block that completes abruptly gives the try statement UpdateEmpty(F, undefined): its
// own value, or undefined, never the value of the try block or the catch block. Each statement
// is put into each place, the exit between the halves of the statement, and run through eval
// once as it is and once with "42; " before the exit.
var bad = [];
function check(source, expected) {
  var found;
  try { found = eval(source); } catch (e) { found = "throw " + e; }
  if (found !== expected) bad.push(source + " => " + found);
}
var statements = [
  ["try { 39 } finally { if (true) { ", " } -2 }"]
];
var places = [
  ["99; do { -99; ", "; -77 } while (false);", "break;"],
  ["99; while (true) { -99; ", "; -77 }", "break;"],
  ["99; for (var i = 0; i < 1; i++) { -99; ", "; -77 }", "break;"],
  ["99; for (var k in { a: 1 }) { -99; ", "; -77 }", "break;"],
  ["99; switch (1) { case 1: -99; ", "; -77 }", "break;"],
  ["99; do { -99; ", "; -77 } while (false);", "continue;"],
  ["99; var n = 0; while (n++ < 1) { -99; ", "; -77 }", "continue;"],
  ["99; for (var i = 0; i < 1; i++) { -99; ", "; -77 }", "continue;"],
  ["99; for (var k in { a: 1 }) { -99; ", "; -77 }", "continue;"],
  ["99; do { switch (1) { case 1: -99; ", "; -77 } } while (false);", "continue;"],
  ["99; L: { -99; ", "; -77 }", "break L;"],
  ["99; L: do { -99; do { -98; ", "; -77 } while (false); -76 } while (false);", "break L;"],
  ["99; L: do { -99; do { -98; ", "; -77 } while (false); -76 } while (false);", "continue L;"]
];
for (var s = 0; s < statements.length; s++) {
  for (var p = 0; p < places.length; p++) {
    var head = places[p][0] + statements[s][0];
    var tail = statements[s][1] + places[p][1];
    check(head + places[p][2] + tail, undefined);
    check(head + "42; " + places[p][2] + tail, 42);
  }
}
bad.length === 0 ? "ok" : bad.join(" | ");
