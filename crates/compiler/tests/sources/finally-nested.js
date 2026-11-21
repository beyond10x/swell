// sec-try-statement: a break out of two finally blocks runs both, inner first; a continue out of a catch block leaves its environment
function f(o) {
  for (var k in o) {
    try {
      try {
        break;
      } finally {
        a;
      }
    } catch (e) {
      (function () { return e; });
      continue;
    } finally {
      b;
    }
  }
}
