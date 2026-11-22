// sec-method-definitions-static-semantics-early-errors: SyntaxError "a setter may not have a rest parameter"
var o = { set s(...v) {} };
