// sec-assignment-operators-runtime-semantics-evaluation: a logical assignment puts its value only when the left value does not decide it; an anonymous function takes the name of an identifier target
a ||= 1;
o.p &&= 2;
o[k] ??= 3;
f ??= function () {};
function g(x) { x ||= 4; }
