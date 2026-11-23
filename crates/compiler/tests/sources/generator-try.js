// sec-try-statement-runtime-semantics-evaluation: a yield inside try, catch and finally blocks, whose return goes through the finally block
function* t() {
  try {
    yield 1;
  } catch (e) {
    yield e;
  } finally {
    yield 3;
  }
}
