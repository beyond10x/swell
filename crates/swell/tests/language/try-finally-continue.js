// sec-try-statement: normal "0ff2f"
var s = "";
for (var i = 0; i < 3; i++) { try { if (i === 1) continue; s += i; } finally { s += "f"; } }
s;
