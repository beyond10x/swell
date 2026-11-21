// sec-arguments-exotic-objects: parses
var f = function () { return function () { return typeof arguments; }; };
