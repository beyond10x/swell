// sec-arrow-function-definitions-static-semantics-early-errors: SyntaxError "lexical declaration of a conflicts with a parameter"
var f = (a) => {
  let a;
};
