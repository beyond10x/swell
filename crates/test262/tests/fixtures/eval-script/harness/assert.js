// Written for the tests of swell-test262: a small assert.js, not test262's.

function assert(mustBeTrue, message) {
  if (mustBeTrue !== true) {
    throw new Test262Error(message || "expected true");
  }
}

assert.sameValue = function (actual, expected, message) {
  if (actual !== expected) {
    throw new Test262Error((message || "") + " expected " + expected + ", got " + actual);
  }
};

// The copy of the refused tree's assert.js, with assert.throws: a case of this tree that throws
// the error it names would pass but for the mark of evalScript.
assert.throws = function (expected, func, message) {
  try {
    func();
  } catch (thrown) {
    if (typeof thrown !== "object" || thrown === null || thrown.constructor !== expected) {
      throw new Test262Error((message || "") + " expected a " + expected.name + ", got " + thrown);
    }
    return;
  }
  throw new Test262Error((message || "") + " expected a " + expected.name + ", nothing was thrown");
};
