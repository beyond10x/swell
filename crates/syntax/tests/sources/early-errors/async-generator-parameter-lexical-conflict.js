// sec-async-generator-function-definitions-static-semantics-early-errors: SyntaxError "lexical declaration of a conflicts with a parameter"
var g = async function* (a) {
  const a = 0;
};
