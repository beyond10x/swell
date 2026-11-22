// sec-block-static-semantics-early-errors: SyntaxError "duplicate lexical declaration of f"
{
  function f() {}
  let f;
}
