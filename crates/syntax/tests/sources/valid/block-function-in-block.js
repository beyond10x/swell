// sec-block: parses
{ function f() {} f(); }
{ function f() {} function g() { return f; } }
{ { function f() {} } function f() {} }
function outer(f) { { function f() {} } var g; { function g() {} } }
try { function t() {} } catch (e) { function c() {} } finally { function z() {} }
try {} catch ({ a }) { function b() {} }
if (a) { function i() {} } else { function i() {} }
for (;;) { function l() {} break; }
var v; let w; { function v() {} function w() {} }
