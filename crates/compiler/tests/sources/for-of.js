// sec-for-in-and-for-of-statements: the iterator kept on the stack, each value put to the target, IteratorClose run as a finally over the body
for (var v of a) v;
for (a.b of c) ;
for ({ p: a[0] } of c) ;
