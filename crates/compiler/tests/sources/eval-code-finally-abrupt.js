// sec-try-statement-runtime-semantics-evaluation: a finally block entered in eval code sets the completion value register to undefined after it saves it, so a break or continue out of it leaves its own value or undefined (UpdateEmpty(F, undefined)), and a normal end puts the saved value back
99; do { -99; try { 39 } finally { 42; break; } } while (false);
do { try { 39 } catch (e) { -1 } finally { continue; } } while (false);
