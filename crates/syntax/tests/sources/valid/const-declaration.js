// sec-let-and-const-declarations: parses
const a = 1, b = a;
function f() {
  const c = b;
  return c;
}
