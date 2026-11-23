// sec-regexpbuiltinexec: normal "2024,11,3,true"
// A RegExp made from a String, as the consumer table's constructions are, its exec and test.
var date = new RegExp("(\\d+)-(\\d+)");
var m = date.exec("on 2024-11");
[m[1], m[2], m.index, date.test("1-2")].join();
