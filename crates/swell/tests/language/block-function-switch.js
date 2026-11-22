// sec-switch-statement-runtime-semantics-evaluation: normal "later/function,function,true,function,outer"
// The CaseBlock is one scope, entered after the discriminant: its functions are made before the
// first clause's test, and every clause sees the same object.
function clauses(x) {
  var seen = [];
  switch (x) {
    case 0:
      seen.push(later());
    case 1:
      seen.push(typeof later);
      function later() { return "later"; }
      break;
    default:
      seen.push(later);
  }
  return seen;
}
var r = [];
r.push(clauses(0).join("/"));
r.push(typeof clauses(2)[0]);
var first;
switch (1) {
  case 1:
    first = f;
  default:
    r.push(first === f);
    function f() {}
}
let g = "outer";
switch (typeof g) {
  case "string":
    r.push(typeof g);
    function g() {}
}
r.push(g);
r.join();
