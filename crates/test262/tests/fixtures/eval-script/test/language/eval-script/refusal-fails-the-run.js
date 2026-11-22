// Written for the tests of swell-test262.
/*---
description: >
  A refusal of the parser inside $262.evalScript is thrown as the SyntaxError it is, and the
  case catches it as one, but the run fails: the engine declined the source, it did not find an
  error of the grammar.
---*/

assert.throws(SyntaxError, function () { $262.evalScript("<!-- x"); });
