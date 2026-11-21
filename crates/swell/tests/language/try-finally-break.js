// sec-try-statement: normal "0ff"
var s = "";
for (var i = 0; i < 3; i++) { try { if (i === 1) break; s += i; } finally { s += "f"; } }
s;
