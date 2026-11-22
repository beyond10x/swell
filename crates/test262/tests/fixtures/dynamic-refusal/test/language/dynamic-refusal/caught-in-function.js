// Written for the tests of swell-test262.
/*---
description: >
  A refusal of the parser inside the Function constructor is thrown as a SyntaxError, which the
  case catches, but the engine counted it, so the run fails.
---*/

try { Function("<!-- x"); } catch (e) { if (!(e instanceof SyntaxError)) throw e; }
