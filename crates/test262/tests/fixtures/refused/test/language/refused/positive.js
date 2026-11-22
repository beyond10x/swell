// Written for the tests of swell-test262.
/*---
description: A case whose source the parser refuses fails.
---*/

var f = 1; <!-- an HTML-like comment
assert.sameValue(f, 1);
