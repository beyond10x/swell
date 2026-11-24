// sec-async-arrow-function-definitions-static-semantics-early-errors: SyntaxError "yield expression in the parameters of an arrow function"
function* g() {
  async (x = yield) => x;
}
