// sec-switch-statement-runtime-semantics-evaluation: a binding of a case block is checked at every use, its clauses being entered by a jump
function f(x) {
  switch (x) {
    case 0:
      let a = 1;
    default:
      return a;
  }
}
