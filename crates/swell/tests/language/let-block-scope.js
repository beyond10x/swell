// sec-block-runtime-semantics-evaluation: normal "1,2,1,3,4,1"
let a = 1;
var seen = [a];
{
  let a = 2;
  seen.push(a);
}
seen.push(a);
function f() {
  { const a = 3; return a; }
}
seen.push(f());
try {
  const a = 4;
  throw a;
} catch (e) {
  let a = e;
  seen.push(a);
}
seen.push(a);
seen.join(",");
