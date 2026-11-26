// sec-generator-function-definitions-static-semantics-early-errors: SyntaxError "super property outside a method"
function* g() { yield super.x; }
