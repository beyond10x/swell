// sec-arrow-function-definitions: parses
var a = () => {};
var b = (x, y) => { return x + y; };
var c = () => { "use strict"; return this; };
var d = () => { return () => { return arguments; }; };
var e = () => { var f = function () { return this; }; return f; };
x => {}
(() => {})
var g = () => {}
var h = 1;
