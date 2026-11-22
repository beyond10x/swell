// sec-switch-statement-static-semantics-early-errors: SyntaxError "lexical declaration of f conflicts with a var or function declaration"
switch (a) {
  case 0:
    function f() {}
  default:
    var f;
}
