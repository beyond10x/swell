// sec-runtime-semantics-evaluateasyncfunctionbody: the start makes the promise and the activation in the last slot, an await suspends and resumes, and the end of the body resolves
async function f(x) {
  var y = await x;
  return y;
}
