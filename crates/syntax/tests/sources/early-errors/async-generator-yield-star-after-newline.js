// sec-async-generator-function-definitions: SyntaxError "unexpected token *"
var g = async function* () {
  yield
  * 1;
};
