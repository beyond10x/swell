// sec-runtime-semantics-evaluateasyncfunctionbody: the defaults of the parameters run after the start, inside the region whose handler rejects the promise
async function d(a = b(), c = a) {
  return await a + c;
}
