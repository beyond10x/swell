// sec-generator-function-definitions-static-semantics-early-errors: SyntaxError "lexical declaration of a conflicts with a parameter"
function* g(a) {
  let a;
}
