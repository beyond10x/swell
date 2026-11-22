// sec-destructuring-assignment: parses
[a, , b = 1, [c, d = 2] = [], { e }, (f), g.h, i[0], ...j] = o;
[] = o;
[, ,] = o;
[...k.l] = o;
[...[m, n]] = o;
[...{ length }] = o;
x = [y] = [z] = o;
for ([p, q] of o) {}
for ([r] in o) {}
for ([s = 1, ...t] of o) {}
f([u] = o, [v, w] = o);
