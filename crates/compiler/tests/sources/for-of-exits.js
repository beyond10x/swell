// sec-iteratorclose: break, return and continue to an outer loop close the iterator; continue of the loop itself does not
function f() {
  outer: for (var a of b) {
    for (var c of d) {
      if (c) continue outer;
      if (a) break;
      if (b) continue;
      return c;
    }
  }
}
