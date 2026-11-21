// sec-delete-operator-runtime-semantics-evaluation: normal "number undefined true false"
// sloppy-only
var o = { a: 1 };
var r;
with (o) { r = typeof a + " " + typeof nothing + " " + delete a + " " + ("a" in o); }
r;
