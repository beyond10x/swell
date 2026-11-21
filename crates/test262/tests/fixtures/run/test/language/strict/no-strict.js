// Written for the tests of swell-test262.
/*---
description: The body of both.js with noStrict, run once in non-strict code, passes.
flags: [noStrict]
---*/

var self = (function () { return this; })();
if (self === undefined) {
  throw new Test262Error("this run is strict");
}
