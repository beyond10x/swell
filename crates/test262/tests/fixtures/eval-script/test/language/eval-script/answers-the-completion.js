// Written for the tests of swell-test262.
/*---
description: >
  $262.evalScript answers the completion value of ScriptEvaluation of its source, after
  ToString of its argument; an error the script throws is thrown to the caller.
---*/

assert.sameValue($262.evalScript("1 + 1;"), 2, "the completion value");
assert.sameValue($262.evalScript("var unused = 1;"), undefined, "a declaration has none");
assert.sameValue($262.evalScript({ toString: function () { return "'text';"; } }), "text", "ToString of the argument");

var thrown;
try {
  $262.evalScript("throw 7;");
} catch (e) {
  thrown = e;
}
assert.sameValue(thrown, 7, "the throw of the script");
