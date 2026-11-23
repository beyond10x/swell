// Written for the tests of swell-test262.
/*---
description: An async case that calls $DONE with an error fails, with the line $DONE printed.
flags: [async]
---*/

$DONE(new Test262Error("x"));
