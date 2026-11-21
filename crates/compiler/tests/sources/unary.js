// sec-unary-operators: delete of a reference and of a value, void, typeof of an unresolvable name, + - ~ !
delete a.b;
delete a[c];
delete x;
delete 1;
void 0;
typeof y;
typeof a.b;
+a;
-a;
~a;
!a;
function f(p) { return delete p; }
