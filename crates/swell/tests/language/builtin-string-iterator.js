// sec-%stringiteratorprototype%.next: normal "x,2,56832,y,true,true,String Iterator"
// String.prototype[Symbol.iterator] (sec-string.prototype-%symbol.iterator%) answers a String
// Iterator whose next answers each code point as a String: a surrogate pair is one value of two
// code units (sec-codepointat); then done with the value undefined. The iterator's
// [Symbol.toStringTag] is "String Iterator".
var it = "x😀y"[Symbol.iterator](), r = [];
var a = it.next(), b = it.next(), c = it.next(), d = it.next();
r.push(a.value, b.value.length, b.value.charCodeAt(1), c.value, d.done, d.value === undefined,
       it[Symbol.toStringTag]);
r.join();
