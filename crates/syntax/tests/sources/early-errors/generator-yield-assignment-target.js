// sec-assignment-operators-static-semantics-early-errors: SyntaxError "invalid assignment target"
function* g() {
  (yield) = 1;
}
