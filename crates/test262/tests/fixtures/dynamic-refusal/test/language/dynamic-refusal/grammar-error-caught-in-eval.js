// Written for the tests of swell-test262.
/*---
description: >
  An error of the grammar inside eval is a SyntaxError the case may catch: no refusal is
  counted, and the run passes.
---*/

var caught = false;
try { eval("var;"); } catch (e) { caught = e instanceof SyntaxError; }
assert(caught, "eval(\"var;\") throws a SyntaxError");
