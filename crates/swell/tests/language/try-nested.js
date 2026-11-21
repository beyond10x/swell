// sec-try-statement: normal "f1"
var s = "";
try { try { throw 1; } finally { s += "f"; } } catch (e) { s += e; }
s;
