// sec-ordinarycallbindthis: normal true
// sloppy-only
function f() { "use strict"; return this; }
f() === undefined;
