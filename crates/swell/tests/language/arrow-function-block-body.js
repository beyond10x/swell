// sec-arrow-function-definitions-runtime-semantics-evaluation: normal true
// A block body runs its statements and answers what its return gives, undefined without one.
var sum = (n) => { var s = 0; for (var i = 1; i <= n; i++) { s += i; } return s; };
var nothing = () => { 1; };
var early = (x) => { if (x) { return "early"; } return "late"; };
var inner = () => { function g() { return 5; } return g(); };
var scoped = () => { let a = 1; { let a = 2; } return a; };
sum(4) === 10 && nothing() === undefined && early(true) === "early" && early(false) === "late"
  && inner() === 5 && scoped() === 1;
