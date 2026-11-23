// sec-switch-statement-static-semantics-early-errors: SyntaxError "duplicate lexical declaration of f"
switch (a) {
  case 0:
    function* f() {}
  default:
    let f;
}
