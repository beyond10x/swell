// Written for the tests of swell-test262.
/*---
description: >
  Passes in non-strict code only: run twice, its strict run fails, so the case fails.
---*/

var self = (function () { return this; })();
if (self === undefined) {
  throw new Test262Error("this run is strict");
}
