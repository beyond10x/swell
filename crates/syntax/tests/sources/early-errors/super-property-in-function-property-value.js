// sec-function-definitions-static-semantics-early-errors: SyntaxError "super property outside a method"
var o = { m() { return 1; }, f: function () { return super.x; } };
