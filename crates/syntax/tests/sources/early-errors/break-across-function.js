// sec-break-statement-static-semantics-early-errors: SyntaxError "break outside of a loop or switch"
while (a) { (function () { break; }); }
