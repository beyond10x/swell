// sec-object-environment-records-hasbinding-n: normal 3
// sloppy-only
// An object with no @@unscopables property: every property, own or inherited, is a binding.
function P() {}
P.prototype.b = 2;
var o = new P();
o.a = 1;
with (o) { a + b; }
