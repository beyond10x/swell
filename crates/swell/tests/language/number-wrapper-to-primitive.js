// sec-ordinarytoprimitive: normal "7|true|3|object"
var n = new Number(2);
(n * new Number(3) + new Number(1)) + "|" + (n == 2) + "|" + (n + 1) + "|" + typeof n;
