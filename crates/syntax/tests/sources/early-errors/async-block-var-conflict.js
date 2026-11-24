// sec-block-static-semantics-early-errors: SyntaxError "lexical declaration of f conflicts with a var or function declaration"
{
  async function f() {}
  var f;
}
