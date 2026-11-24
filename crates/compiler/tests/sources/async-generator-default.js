// sec-asyncgenerator-definitions-evaluatebody: the defaults of the parameters run before the async generator object is made, outside the region whose handler settles its requests
async function* d(a = 1, b = a) {
  yield a + b;
}
