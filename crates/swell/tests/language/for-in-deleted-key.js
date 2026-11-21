// sec-enumerate-object-properties: normal "ac"
var o = { a: 1, b: 2, c: 3 };
var s = "";
for (var k in o) { s += k; delete o.b; }
s;
