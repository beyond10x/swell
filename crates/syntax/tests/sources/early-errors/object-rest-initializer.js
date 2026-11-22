// sec-destructuring-binding-patterns: SyntaxError "a rest property may not have an initializer"
var { ...a = 1 } = o;
