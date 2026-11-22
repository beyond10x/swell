// sec-arrow-function-definitions-runtime-semantics-evaluation: normal true
// An expression body is the value the call returns; it takes a whole AssignmentExpression.
var add = (a, b) => a + b;
var pick = x => x ? "yes" : "no";
var wrap = (x) => ({ value: x });
var curried = a => b => c => a + b + c;
var none = () => undefined;
var trailing = (a, b,) => b;
var seq = (a => a, 7);
add(2, 3) === 5 && pick(0) === "no" && pick(1) === "yes" && wrap(4).value === 4
  && curried(1)(2)(3) === 6 && none() === undefined && trailing(1, 2) === 2 && seq === 7
  && add.length === 2 && trailing.length === 2;
