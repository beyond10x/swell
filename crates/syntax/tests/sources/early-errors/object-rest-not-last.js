// sec-destructuring-binding-patterns: SyntaxError "a rest property must be the last property of its pattern"
var { ...a, b } = o;
