// sec-enumerate-object-properties: normal "bca"
var p = { a: 1, b: 2 };
var o = { __proto__: p, b: 3, c: 4 };
var s = "";
for (var k in o) s += k;
s;
