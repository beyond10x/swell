// Written for the tests of swell-test262.
/*---
description: A raw case runs once, as it is, with no harness file and not in strict code.
flags: [raw]
---*/

if (typeof assert !== "undefined" || typeof Test262Error !== "undefined") {
  throw new Error("a harness file ran before a raw case");
}
if ((function () { return this; })() === undefined) {
  throw new Error("a raw case ran in strict code");
}
