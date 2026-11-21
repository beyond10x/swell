// sec-for-in-and-for-of-statements: the object, an enumerator kept on the stack, each key put to the target
for (var k in o) k;
for (a.b in o) ;
for (a[0] in o) ;
