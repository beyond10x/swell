// sec-update-expressions: normal "3,1,3,3,4"
var a = 1;
var b = a++;
var c = ++a;
var o = { p: 1 };
o.p++;
++o["p"];
var s = "5";
s--;
a + "," + b + "," + c + "," + o.p + "," + s;
