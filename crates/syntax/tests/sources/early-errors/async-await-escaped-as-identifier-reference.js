// sec-identifiers-static-semantics-early-errors: SyntaxError "await may not be an identifier in an async function"
async function f() {
  void \u0061wait;
}
