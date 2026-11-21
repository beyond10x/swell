// Written for the tests of swell-test262.
/*---
description: >
  A regular expression literal is refused when it is evaluated, before slice 5; the case catches
  the SyntaxError, but the engine counted the refusal, so the run fails.
---*/

try { /a/; } catch (e) { if (!(e instanceof SyntaxError)) throw e; }
