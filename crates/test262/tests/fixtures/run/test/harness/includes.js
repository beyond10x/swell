// Written for the tests of swell-test262.
/*---
description: The includes run after assert.js and sta.js, in the order the case names them.
includes: [order-a.js, order-b.js]
---*/

assert.sameValue(order.join(), "a,b");
