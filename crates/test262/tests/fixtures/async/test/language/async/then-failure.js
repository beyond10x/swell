// Written for the tests of swell-test262.
/*---
description: >
  An async case whose rejection handler is $DONE fails, with the line $DONE printed.
flags: [async]
---*/

Promise.reject(new Test262Error("x")).then(null, $DONE);
