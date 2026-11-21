// sec-arguments-exotic-objects: refused "reference to arguments inside a function is not supported before slice 2"
var f = function () { return function () { return typeof arguments; }; };
