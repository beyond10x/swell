// sec-break-statement: a jump past the loop or switch; out of a for-in it lands on the Pop of the enumerator
while (a) break;
for (var k in o) { break; }
switch (b) { case 1: break; }
