// sec-strict-mode-code: the code of a class in non-strict code is strict, between SetStrict true and false; a class in its key emits neither; an anonymous class under a computed key is named before its elements; a yield* in a key has a strict handler, and the return of a yield there puts the strictness back before the finally
var o = { [k]: class { [class { [k]() {} }]() {} } };
function* g() { try { class C { [yield* h()]() {} [yield 1]() {} } } finally { k = 1; } }
