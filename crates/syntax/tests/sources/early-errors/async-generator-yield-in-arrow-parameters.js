// sec-arrow-function-definitions-static-semantics-early-errors: SyntaxError "yield expression in the parameters of an arrow function"
async function* g() {
  (a = yield) => a;
}
