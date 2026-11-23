// sec-generator-function-definitions-static-semantics-early-errors: SyntaxError "lexical declaration of a conflicts with a parameter"
x = {
  *g(a) {
    let a;
  },
};
