// Written for the tests of swell-test262: a small assert.js, not test262's.

function assert(mustBeTrue, message) {
  if (mustBeTrue !== true) {
    throw new Test262Error(message || "expected true");
  }
}
