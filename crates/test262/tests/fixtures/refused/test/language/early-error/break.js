// Written for the tests of swell-test262.
/*---
description: >
  A negative parse case with an early error of the ES5 grammar (a break outside of a loop or
  switch) passes.
negative:
  phase: parse
  type: SyntaxError
---*/

$DONOTEVALUATE();

break;
