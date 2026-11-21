// sec-continue-statement-runtime-semantics-evaluation: normal 3
var n = 0;
outer: for (var i = 0; i < 3; i++) {
  for (var j = 0; j < 3; j++) {
    if (j === 1) continue outer;
    n += 1;
  }
  n += 100;
}
n;
