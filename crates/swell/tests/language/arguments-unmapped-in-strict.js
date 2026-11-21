// sec-createunmappedargumentsobject: normal "1 2 3"
// strict-only
function f(a) { arguments[0] = 2; var seen = a + " " + arguments[0]; a = 3; return seen + " " + a; }
f(1);
