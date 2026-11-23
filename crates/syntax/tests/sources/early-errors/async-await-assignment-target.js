// sec-assignment-operators-static-semantics-early-errors: SyntaxError "invalid assignment target"
async function f() {
  (await x) = 1;
}
