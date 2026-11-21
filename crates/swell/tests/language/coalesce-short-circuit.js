// sec-binary-logical-operators-runtime-semantics-evaluation: normal "0:0,,7,8,false"
var calls = 0;
function g() { calls += 1; return 5; }
var r = "" + (0 ?? g()) + "," + ("" ?? g()) + "," + (null ?? 7) + "," + (undefined ?? 8) + "," + (false ?? g());
calls + ":" + r;
