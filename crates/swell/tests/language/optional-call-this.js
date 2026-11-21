// sec-optional-chaining-chain-evaluation: normal "4444undefined"
var o = { v: 4, m: function () { return this.v; } };
var p = null;
"" + o?.m() + o.m?.() + (o?.m)() + o?.["m"]() + p?.m();
