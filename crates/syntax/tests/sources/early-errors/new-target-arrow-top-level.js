// sec-scripts-static-semantics-early-errors: SyntaxError "new.target outside a function"
var f = (a = 1) => () => { return new.target; };
