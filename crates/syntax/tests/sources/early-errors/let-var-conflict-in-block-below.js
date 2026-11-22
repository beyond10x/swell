// sec-block-static-semantics-early-errors: SyntaxError "lexical declaration of a conflicts with a var or function declaration"
function f() {
  { var a; }
  let a;
}
