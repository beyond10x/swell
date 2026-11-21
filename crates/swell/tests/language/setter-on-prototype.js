// sec-ordinaryset: normal "6,false"
var p = { set v(x) { this.w = x * 2; } };
var o = { __proto__: p };
o.v = 3;
o.w + "," + ("w" in p);
