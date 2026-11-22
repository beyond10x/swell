// sec-for-statement: parses
for (let i = 0, j; i < 1; i++) { let i = 2; }
for (const k = 0; false; ) {}
for (let k in {}) { var v = k; }
for (const k in {}) {}
for (var i = 0; i < 1; i++) {}
