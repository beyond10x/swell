// sec-let-and-const-declarations: parses
// sloppy-only
var let = 1;
let = 2;
let;
let.x;
if (false) let // ASI
x = 1;
while (false) let
{}
for (let in {}) {}
for (let; false; ) {}
