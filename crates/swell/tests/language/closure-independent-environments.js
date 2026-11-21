// sec-environment-records: normal 21
function mk(x) { return function () { return x; }; }
var a = mk(1), b = mk(2);
a() + b() * 10;
