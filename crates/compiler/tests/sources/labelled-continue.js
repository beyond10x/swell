// sec-continue-statement-runtime-semantics-evaluation: continue to a label goes to the next iteration of the labelled loop, through a finally block
a: for (;;) { try { continue a; } finally { b; } }
