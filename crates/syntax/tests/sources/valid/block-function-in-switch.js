// sec-switch-statement: parses
switch (a) {
  case 1:
    function f() {}
  case 2:
    function g() { return f; }
    break;
  default:
    function h() {}
}
switch (a) { default: function f() {} }
let k; switch (a) { case 0: function k() {} }
