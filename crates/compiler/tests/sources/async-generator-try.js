// sec-asyncgeneratorunwrapyieldresumption: a yield inside try, catch and finally blocks, whose return completion is awaited and goes through the finally block, and a bare return that is not awaited
var e = async function* () {
  try {
    yield 1;
  } catch (e) {
    yield e;
  } finally {
    yield 3;
  }
  return;
};
