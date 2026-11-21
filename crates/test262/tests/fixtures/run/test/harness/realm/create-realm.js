// Written for the tests of swell-test262.
/*---
description: $262.createRealm gives a new realm with a fresh global, every call another one.
---*/

assert.sameValue($262.global, this, "$262.global is the global object");
var other = $262.createRealm();
assert.sameValue(typeof other.global, "object");
assert(other.global !== this, "the new realm has its own global object");
assert(other.global.Object !== Object, "the new realm has its own intrinsics");
assert(other.global.$262 === other, "the new realm has its own $262");
other.global.marker = 1;
assert.sameValue(typeof marker, "undefined", "a property of the new global is not one of this");
var third = other.createRealm();
assert(third.global !== other.global, "a second call gives another realm");
assert(third.global !== this, "a second call gives another realm");
