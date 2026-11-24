// sec-block-static-semantics-early-errors: SyntaxError "duplicate lexical declaration of f"
{
  async function f() {}
  function f() {}
}
