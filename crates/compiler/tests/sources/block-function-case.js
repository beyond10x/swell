// sec-switch-statement-runtime-semantics-evaluation: the functions of a case block are made after the discriminant, before the first test, and shared by every clause
function g(x) {
  switch (x) {
    case 0:
      f();
    case 1:
      function f() {}
  }
}
