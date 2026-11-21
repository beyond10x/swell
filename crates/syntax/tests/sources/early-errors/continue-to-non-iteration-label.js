// sec-continue-statement-static-semantics-early-errors: SyntaxError "continue to label a, which labels no iteration statement"
a: { while (x) { continue a; } }
