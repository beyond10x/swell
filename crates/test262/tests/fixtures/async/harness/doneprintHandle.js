// Written for the tests of swell-test262: a small doneprintHandle.js, not test262's. $DONE
// prints the two lines test262's file prints: Test262:AsyncTestComplete when called with no
// error, Test262:AsyncTestFailure: and the error otherwise.

function $DONE(error) {
  if (error) {
    if (typeof error === "object" && error !== null && "name" in error) {
      print("Test262:AsyncTestFailure:" + error.name + ": " + error.message);
    } else {
      print("Test262:AsyncTestFailure:Test262Error: " + String(error));
    }
  } else {
    print("Test262:AsyncTestComplete");
  }
}
