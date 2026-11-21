// sec-try-statement: parses
try { a(); } catch (e) { b(e); }
try { a(); } finally { c(); }
try {} catch (e) { var e; } finally {}
