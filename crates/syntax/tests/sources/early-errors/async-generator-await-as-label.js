// sec-identifiers-static-semantics-early-errors: SyntaxError "await may not be an identifier in an async function"
var g = async function* () {
  \u0061wait: ;
};
