// sec-updateempty: a script's completion value through a loop, a break through a finally block and a break out of one
1;
while (a) { try { 2; break; } finally { 3; } }
do { try {} finally { break; } } while (b);
