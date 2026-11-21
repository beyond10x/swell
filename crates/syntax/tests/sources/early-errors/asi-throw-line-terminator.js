// sec-rules-of-automatic-semicolon-insertion: SyntaxError "line terminator after throw"
function f() {
  throw
  new Error();
}
