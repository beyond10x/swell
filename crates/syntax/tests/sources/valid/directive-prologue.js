// sec-directive-prologues-and-the-use-strict-directive: parses
"a directive";
'another';
function f() { "use strict"; return 1; }
function g() { "not strict"; "use strict"; }
