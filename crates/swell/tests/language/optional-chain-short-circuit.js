// sec-optional-chaining-chain-evaluation: normal "0:undefined,undefined,undefined,undefined,undefined,true,3,undefined"
var calls = 0;
function f() { calls += 1; return "k"; }
var a = null;
var o = { b: { c: 3 } };
var r = "" + a?.b + "," + a?.[f()] + "," + a?.b.c.d + "," + a?.(f()) + "," + typeof a?.b + "," + delete a?.b + "," + o?.b.c + "," + o.x?.y;
calls + ":" + r;
