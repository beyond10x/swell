// sec-performeval: normal "3 undefined undefined 5 undefined"
var a = eval("1; 2; 3;");
var b = eval("");
var c = eval("var q = 4;");
var d = eval("if (true) { 5; } else { 6; }");
var e = eval();
a + " " + b + " " + c + " " + d + " " + e;
