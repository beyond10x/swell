// sec-arrow-function-definitions: parses
var a = (x = 1) => x;
var b = (...r) => r;
var c = (x, y = x, ...r) => { return r; };
var d = (x = (y = 1) => y) => x;
var e = (x = function () {}) => x;
var f = (x, y = 2,) => x + y;
