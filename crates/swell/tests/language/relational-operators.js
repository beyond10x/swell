// sec-relational-operators: normal "true,true,true,false,false,true"
(1 < 2) + "," + ("a" < "b") + "," + ("10" < "9") + "," + (10 < "9") + "," + (NaN <= 1) + "," + ("a" in { a: 1 });
