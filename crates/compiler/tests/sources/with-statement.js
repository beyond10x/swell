// sec-with-statement-runtime-semantics-evaluation: the object environment around the body, each name in it looked up by name, the environment popped after it
var o = { a: 1 };
with (o) { a = b + 1; a++; typeof c; delete d; }
