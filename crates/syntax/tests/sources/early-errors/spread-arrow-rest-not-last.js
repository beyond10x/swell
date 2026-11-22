// sec-arrow-function-definitions-static-semantics-early-errors: SyntaxError "a rest element must be the last element of its pattern"
var af = ([...a, b]) => a;
