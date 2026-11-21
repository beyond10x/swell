// sec-labelled-statements: parses
outer: for (var i = 0; i < 2; i++) {
  inner: while (true) {
    if (i) continue outer;
    break inner;
  }
}
