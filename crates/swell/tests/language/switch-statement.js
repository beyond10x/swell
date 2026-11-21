// sec-switch-statement: normal "onefewmany"
function f(x) {
  switch (x) {
    case 1: return "one";
    case 2:
    case 3: return "few";
    default: return "many";
  }
}
f(1) + f(3) + f(9);
