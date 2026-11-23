// sec-regexp.prototype.test: normal true
// A class of \u escapes, as the consumer table's row 2 has one, and its call of test.
var controls = /[\u0000-\u001f\u007f-\u009f]/;
controls.test("a\u0001b") && controls.test("\u009f") && !controls.test("abc") &&
  controls.lastIndex === 0;
