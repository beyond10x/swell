// sec-functiondeclarationinstantiation: the declarations are instantiated before the body runs, the last of a name wins
function g() {
  return h;
  function h() {}
  function h(a) {}
}
