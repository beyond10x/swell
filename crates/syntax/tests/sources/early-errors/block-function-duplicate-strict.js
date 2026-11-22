// sec-block-static-semantics-early-errors: SyntaxError "duplicate lexical declaration of f"
"use strict";
{
  function f() {}
  function f() {}
}
