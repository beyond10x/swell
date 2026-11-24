// sec-arrow-function-definitions-static-semantics-early-errors: SyntaxError "await in the parameters of an arrow function"
async function* g() {
  (a = await 1) => a;
}
