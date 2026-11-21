// Written for the tests of swell-test262.
/*---
description: Passes in strict code only; with onlyStrict it runs once, strict, and passes.
flags: [onlyStrict]
---*/

var self = (function () { return this; })();
if (self !== undefined) {
  throw new Test262Error("this run is not strict");
}
