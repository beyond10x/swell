// sec-createunmappedargumentsobject: a strict function gets an unmapped arguments object and its parameters keep their local slots
function f(a) { "use strict"; return arguments.length + a; }
