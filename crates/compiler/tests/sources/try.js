// sec-try-statement: catch and finally handlers over their ranges; the finally block gets a completion record
try { a; } catch (e) { e; }
try { b; } finally { c; }
try { d; } catch (e) { } finally { }
