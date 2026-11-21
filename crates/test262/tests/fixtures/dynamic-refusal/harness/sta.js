// Written for the tests of swell-test262: a small sta.js, not test262's.

function Test262Error(message) {
  this.message = message || "";
}

Test262Error.prototype.toString = function () {
  return "Test262Error: " + this.message;
};

function $DONOTEVALUATE() {
  throw "Test262: This statement should not be evaluated.";
}
