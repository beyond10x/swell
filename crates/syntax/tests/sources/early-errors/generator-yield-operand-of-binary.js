// sec-generator-function-definitions: SyntaxError "unexpected number 4"
var g = function* () {
  yield 3 + yield 4;
};
