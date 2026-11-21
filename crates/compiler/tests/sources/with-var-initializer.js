// sec-variable-statement-runtime-semantics-evaluation: a var initializer inside a with body puts by name, and every binding of the scopes around it, the catch parameter included, has a named slot
function f(o) { var x; try { throw 0; } catch (e) { with (o) { var x = e; } } return x; }
