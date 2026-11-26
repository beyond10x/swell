// sec-function-definitions-static-semantics-early-errors: SyntaxError "super property outside a method"
var f = function (a = super.x) { return a; };
