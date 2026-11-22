// sec-destructuring-binding-patterns: parses
var { a, b: c, "d": e, 1: f, g = 1, h: { i = 2, j } = {} } = o;
let { k, l: { m } } = o, { n } = o;
const { p = function () {}, q: r = () => 0 } = o;
for (var { s } = o; s; ) {}
for (let { t } = o; t; ) {}
for (var { u } in o) {}
for (let { v, w } in o) {}
for (const { length } in o) {}
try {} catch ({ x, y = x }) {}
var {} = o;
