// sec-block-static-semantics-early-errors: SyntaxError "duplicate lexical declaration of f"
"use strict";
{
  const f = 0;
  function f() {}
}
