// sec-async-function-definitions-static-semantics-early-errors: SyntaxError "lexical declaration of a conflicts with a parameter"
x = {
  async m(a) {
    let a;
  },
};
