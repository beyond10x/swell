// sec-for-in-and-for-of-statements: parses
for (p in o) {}
for (var q in o) { if (q) continue; }
for (o.p in o) break;
for (o[0] in { a: 1 }) ;
for (var r in "a" in o ? o : {}) {}
