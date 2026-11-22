// sec-runtime-semantics-arrayaccumulation: a spread appends its values to the array after the elements before it, an elision after a spread grows the length; a call, a method call and new with a spread build one argument Array
function f(a, o) { var x = [0, ...a, , 1]; o.m(...a, 2); f(...a); return new o(1, ...a); }
