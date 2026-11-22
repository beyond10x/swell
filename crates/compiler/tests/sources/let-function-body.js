// sec-functiondeclarationinstantiation: the top level of a function body is a scope of its own, entered before the functions it declares are made
function f() {
  let x = 1;
  function g() { return x; }
  return g();
}
