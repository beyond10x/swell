// sec-object.prototype.tostring: normal "[object Arguments]"
function f() { return Object.prototype.toString.call(arguments); }
f(1);
