// sec-block-static-semantics-early-errors: parses
let a = 0;
{ let a = 1; { const a = 2; } }
try { let a = 3; } catch (e) { let b = e; } finally { const a = 4; }
function f(p) { { let p = 5; } let a = p; return a; }
(function g() { let g = 6; return g; });
