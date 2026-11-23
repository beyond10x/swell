// sec-runtime-semantics-evaluategeneratorbody: the defaults of the parameters run before the generator object is made
function* d(a = 1, b = a) {
  yield a + b;
}
