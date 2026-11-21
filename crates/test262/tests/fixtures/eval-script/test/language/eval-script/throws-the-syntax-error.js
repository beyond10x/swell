// Written for the tests of swell-test262.
/*---
description: >
  $262.evalScript of a source with an error of the grammar throws the first error ParseScript
  gives, a SyntaxError of the realm of the function, and runs nothing of the source.
---*/

var caught;
try {
  $262.evalScript("ranBeforeTheError = 1; var;");
} catch (e) {
  caught = e;
}
assert(caught !== undefined, "evalScript threw");
assert.sameValue(caught.constructor, SyntaxError, "a SyntaxError of this realm");
assert.sameValue(typeof ranBeforeTheError, "undefined", "nothing of the source ran");

var other = $262.createRealm();
var otherCaught;
try {
  other.evalScript("var;");
} catch (e) {
  otherCaught = e;
}
assert(otherCaught !== undefined, "the other realm's evalScript threw");
assert.sameValue(otherCaught.constructor, other.global.SyntaxError, "a SyntaxError of the other realm");
assert(otherCaught.constructor !== SyntaxError, "not one of this realm");
