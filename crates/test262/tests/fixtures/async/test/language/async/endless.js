// Written for the tests of swell-test262.
/*---
description: >
  An async case whose job queues a job like itself ends the checkpoint at its step budget.
flags: [async]
---*/

function f() { Promise.resolve().then(f); }
f();
