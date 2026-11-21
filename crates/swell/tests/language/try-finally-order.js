// sec-try-statement: normal "rtf"
var s = "";
function f() { try { s += "t"; return "r"; } finally { s += "f"; } }
f() + s;
