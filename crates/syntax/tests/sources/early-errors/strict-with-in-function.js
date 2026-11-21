// sec-with-statement-static-semantics-early-errors: SyntaxError "with statement in strict mode code"
function f() {
  "use strict";
  with (o) {}
}
