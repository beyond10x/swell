// sec-blockdeclarationinstantiation: normal "1,120,true,block,try,catch0,finally"
// Each function of a Block is made before the Block's first statement: called above its
// declaration, by itself and by its sibling, in a block, a try, a catch and a finally block.
var r = [];
{
  r.push(one());
  function one() { return 1; }
}
{
  function fact(n) { return n <= 1 ? 1 : n * fact(n - 1); }
  r.push(fact(5));
}
{
  r.push(even(10));
  function even(n) { return n === 0 ? true : odd(n - 1); }
  function odd(n) { return n === 0 ? false : even(n - 1); }
}
function g() {
  var out = [];
  { out.push(b()); function b() { return "block"; } }
  try { out.push(t()); function t() { return "try"; } } catch (e) {}
  try { throw 0; } catch (e) { out.push(c(e)); function c(x) { return "catch" + x; } }
  try {} finally { out.push(z()); function z() { return "finally"; } }
  return out.join();
}
r.push(g());
r.join();
