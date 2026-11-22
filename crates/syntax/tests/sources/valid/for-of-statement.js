// sec-for-in-and-for-of-statements: parses
for (a of b) {}
for (var c of d) ;
for (o.p of q) break;
for (o[0] of [1]) ;
for (x of y = z) {}
for (x of (y, z)) ;
for (var of of of) ;
outer: for (e of f) { for (g of h) continue outer; }
