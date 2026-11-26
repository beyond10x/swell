// sec-scripts-static-semantics-early-errors: SyntaxError "super property outside a method"
var o = { m() { var f = x => 1; function g() {} var h = { n() { return super.n; } }; return super.x; } };
super.y;
