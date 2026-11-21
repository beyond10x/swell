// sec-variable-statement: a declarator with an initializer resolves its name, evaluates the value, then puts it
var a, b = 1;
function f(x) {
  var c = x, d;
  return c;
}
