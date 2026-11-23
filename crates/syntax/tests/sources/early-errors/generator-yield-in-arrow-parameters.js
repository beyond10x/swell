// sec-arrow-function-definitions-static-semantics-early-errors: SyntaxError "yield expression in the parameters of an arrow function"
function* g() {
  (x = yield) => {};
}
