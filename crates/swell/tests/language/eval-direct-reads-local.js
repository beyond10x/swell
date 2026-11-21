// sec-function-calls-runtime-semantics-evaluation: normal 3
function f(a) { var b = 2; return eval("a + b"); }
f(1);
