// sec-for-in-and-for-of-statements: parses
async function f(o, list) {
  for await (var a of list) ;
  for await (let b of list) { b; }
  for await (const c of list) c;
  for await (var { d, e: [g] } of list) ;
  for await (let [h, ...i] of list) ;
  for await (const { j = 1, ...k } of list) ;
  for await (o.x of list) ;
  for await (o[0] of await list) ;
  for await ([o.y, o.z] of list) ;
  for await ({ w: o.w } of list) ;
  for await (async of list) ;
  outer: for await (var m of list) {
    for await (var n of list) {
      if (n) continue outer;
      break outer;
    }
    continue;
  }
}
var g = async function () { for await (x of y) ; };
var h = async () => { for await (x of y) break; };
var p = { async q() { for await (x of y) return x; } };
