// sec-continue-statement: normal 6
var s = 0;
for (var i = 0; i < 5; i++) { if (i % 2) continue; s += i; }
s;
