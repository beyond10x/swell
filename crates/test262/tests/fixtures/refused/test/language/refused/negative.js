// Written for the tests of swell-test262.
/*---
description: >
  A negative parse case whose source the parser refuses fails, though the refusal is a
  SyntaxError: the engine declined the file, it did not find the error the case names.
negative:
  phase: parse
  type: SyntaxError
---*/

$DONOTEVALUATE();

var f = a => a;
