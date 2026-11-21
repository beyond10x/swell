// sec-for-statement: parses
for (;;) { break; }
for (i = 0; i < 3; i++) {}
for (var i = 0, j = 10; i < j; i++, j--) continue;
for (var k = ("a" in o) ? 1 : 0; k < 1; k++) {}
for (x = a ? b in c : d; false;) {}
