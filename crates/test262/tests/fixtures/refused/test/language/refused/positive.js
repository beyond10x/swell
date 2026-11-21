// Written for the tests of swell-test262.
/*---
description: A case whose source the parser refuses fails.
---*/

var f = a => a;
assert.sameValue(f(1), 1);
