// Written for the tests of swell-test262.
/*---
description: A negative case of the parse phase.
negative:
  phase: parse
  type: SyntaxError
flags: [module]
---*/

$DONOTEVALUATE();

export default 1 +;
