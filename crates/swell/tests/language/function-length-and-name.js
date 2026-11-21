// sec-ordinaryfunctioncreate: normal "3foo,v"
function foo(a, b, c) {}
var v = function () {};
foo.length + foo.name + "," + v.name;
