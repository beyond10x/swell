// Written for the tests of swell-test262.
/*---
description: An async case whose then callback calls $DONE passes.
flags: [async]
---*/

Promise.resolve().then(function () { $DONE(); });
