// sec-block-runtime-semantics-evaluation: normal "function,outer,0;1;2,true"
// A function of a Block is a binding of the Block alone, made anew at each entry. The `let`
// around the Block keeps a name B.3.2 would bind as a `var` out of its reach, so the answer is
// the same in non-strict code with Annex B.
let f = "outer";
var seen = [];
{
  seen.push(typeof f);
  function f() {}
}
seen.push(f);
var made = [];
for (var i = 0; i < 3; i++) {
  let j = i;
  made.push(g);
  function g() { return j; }
}
seen.push(made[0]() + ";" + made[1]() + ";" + made[2]());
seen.push(made[0] !== made[1]);
seen.join();
