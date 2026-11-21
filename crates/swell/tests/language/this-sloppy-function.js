// sec-ordinarycallbindthis: normal true
// sloppy-only
function f() { return this; }
f() === this;
