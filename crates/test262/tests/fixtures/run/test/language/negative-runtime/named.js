// Written for the tests of swell-test262.
/*---
description: A throw of an object whose constructor's name is the named type passes.
negative:
  phase: runtime
  type: Test262Error
---*/

throw new Test262Error("the named error");
