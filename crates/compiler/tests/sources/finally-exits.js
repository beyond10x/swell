// sec-try-statement: return, break and continue out of a try block run its finally block first
function f() {
  while (a) {
    try {
      if (b) return 1;
      if (c) break;
      continue;
    } finally {
      d;
    }
  }
}
function g(o) {
  try {
    for (var k in o) return k;
  } finally {}
}
