// sec-array.prototype.push: normal "true2,1,2"
var a = [];
var n = a.push(1, 2);
"" + Array.isArray(a) + n + "," + a.join();
