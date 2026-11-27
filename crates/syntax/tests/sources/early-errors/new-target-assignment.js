// sec-assignment-operators-static-semantics-early-errors: SyntaxError "invalid assignment target"
function f() {
  new.target = 1;
}
