// Written for the tests of swell-test262.
/*---
description: >
  A source nested deeper than the parser_nesting of the runner (1000) inside $262.evalScript is
  thrown as the SyntaxError of the nesting limit, and the case catches it as one, but the run
  fails: the limit is not an error of the grammar. It is not counted as a refusal.
---*/

var source = "";
for (var i = 0; i < 2000; i++) {
  source += "(";
}
source += "0";
for (var j = 0; j < 2000; j++) {
  source += ")";
}
assert.throws(SyntaxError, function () { $262.evalScript(source); });
