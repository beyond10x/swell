// sec-arrow-function-definitions: parses
var a = x => x;
var b = () => 0;
var c = (x, y) => x + y;
var d = (x, y,) => x;
var e = (x) => ({ x: x });
var f = x => y => x + y;
var g = x => x ? 1 : 2;
var h = [x => x, () => 0];
var i = a ? x => x : y => y;
k(x => x, () => 0);
var j = async => async;
var l = x => x, m = 1;
n = (x) => x;
(() => 0)();
