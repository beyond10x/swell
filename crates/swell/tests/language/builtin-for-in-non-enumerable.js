// sec-enumerate-object-properties: normal "abc"
// String keys in the order they were created (sec-ordinaryownpropertykeys), the non-enumerable one skipped.
var o = { a: 1, b: 2 };
Object.defineProperty(o, "hidden", { value: 3, enumerable: false });
Object.defineProperty(o, "c", { value: 4, enumerable: true });
var s = "";
for (var k in o) s += k;
s;
