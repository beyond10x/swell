// sec-method-definitions: normal 7
var o = { get x() { return 5; }, set x(v) { this.y = v; } };
o.x = 2;
o.x + o.y;
