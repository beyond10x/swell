// sec-function-definitions: parses
function f(...rest) {}
function g(a, b = 1, ...rest) { return rest; }
var h = function (...args) { return args.length; };
var o = { m(a, ...r) {} };
