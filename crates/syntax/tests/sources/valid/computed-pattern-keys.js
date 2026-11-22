// sec-destructuring-binding-patterns: parses
var { [a]: b, [c]: { d } = {} } = o; ({ [e]: f.g, [h]: i = 1 } = o);
