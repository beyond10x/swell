// sec-labelled-statements-runtime-semantics-labelledevaluation: break to a label leaves the labelled statement; a labelled loop is left by its own exit
a: { b; break a; c; }
d: while (e) { while (f) break d; }
