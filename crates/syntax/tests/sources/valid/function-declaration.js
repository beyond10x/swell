// sec-function-definitions: parses
function f() {}
function g(a, b, c) { function inner(d) { return d; } return inner(a); }
function h(a, b) { var a; }
