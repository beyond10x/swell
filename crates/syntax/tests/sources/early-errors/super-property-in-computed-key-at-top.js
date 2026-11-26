// sec-scripts-static-semantics-early-errors: SyntaxError "super property outside a method"
var o = { [super.x]() { return 1; } };
