// sec-identifiers-static-semantics-early-errors: SyntaxError "yield may not be an identifier in a generator function"
function* g() {
  yield: ;
}
