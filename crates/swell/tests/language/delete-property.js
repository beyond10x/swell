// sec-delete-operator: normal "true,false"
var o = { a: 1 };
(delete o.a) + "," + ("a" in o);
