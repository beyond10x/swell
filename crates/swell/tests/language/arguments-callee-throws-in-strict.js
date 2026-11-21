// sec-createunmappedargumentsobject: throw TypeError
// strict-only
function f() { return arguments.callee; }
f();
