// sec-destructuring-assignment: parses
var a, b, c, o = {};
({ a, b: c } = o);
({ a = 1, b: o.p = 2, c: o["q"] } = o);
({ a: (b), b: (o.r) } = o);
({ a: { b, c } = {} } = o);
x = { a } = o;
for ({ a, b = 1 } in o) {}
for ({ length: o.n } in o) {}
({} = o);
({ __proto__: a, __proto__: b } = o);
