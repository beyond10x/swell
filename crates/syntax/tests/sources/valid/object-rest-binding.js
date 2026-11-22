// sec-destructuring-binding-patterns: parses
var { a, ...b } = o; let { [k]: c, ...d } = o; function f({ e, ...g }) {} var h = ({ ...i }) => i; try {} catch ({ ...j }) {}
