// sec-for-in-and-for-of-statements: normal "abc"
var o = { a: 1, b: 2, c: 3 };
var k, s = "";
for (k in o) s += k;
s;
