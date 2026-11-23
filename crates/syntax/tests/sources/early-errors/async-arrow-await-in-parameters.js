// sec-async-arrow-function-definitions-static-semantics-early-errors: SyntaxError "await in the parameters of an arrow function"
async function f() {
  async (x = await 1) => x;
}
