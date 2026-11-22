// sec-iteratorclose: a for-of inside a try with a finally block; a return passes the close of the iterator, then the finally block
function g(o) {
  try {
    for (var k of o) return k;
  } finally {
    o;
  }
}
