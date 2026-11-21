// Written for the tests of swell-test262.
/*---
description: A throw of another type than the named one fails.
negative:
  phase: runtime
  type: Test262Error
---*/

throw new TypeError("not the named error");
