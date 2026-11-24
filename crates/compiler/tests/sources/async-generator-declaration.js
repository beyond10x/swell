// sec-asyncgenerator-definitions-evaluatebody: an async generator declaration starts after FunctionDeclarationInstantiation, awaits the operand of each yield, dispatches on the tag of its resumption, awaits the value of a return completion and of a return statement, and settles its requests at the end of the body
async function* g(a) {
  var b = yield a;
  yield;
  await b;
  return b;
}
