// Written for the tests of swell-test262.
/*---
description: An async case that never calls $DONE fails once the queue is empty.
flags: [async]
---*/

var called = false;
