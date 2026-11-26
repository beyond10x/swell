// sec-exp-operator: SyntaxError "a unary expression as the left operand of ** needs parentheses"
async function f() {
  x = await a ** 2;
}
