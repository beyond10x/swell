// sec-%arrayiteratorprototype%.next: normal "a,b,true,true,0,1,true,0:a,1:b,true,true,Array Iterator"
// Array.prototype.values, keys and entries answer an Array Iterator (sec-createarrayiterator):
// each next answers the next value, index or [index, value] pair, then done with the value
// undefined. Array.prototype[Symbol.iterator] is the function object values
// (sec-array.prototype-%symbol.iterator%), and the iterator's [Symbol.toStringTag] is
// "Array Iterator".
var a = ["a", "b"];
var v = a.values(), k = a.keys(), e = a.entries();
var r = [v.next().value, v.next().value, v.next().done, v.next().value === undefined,
         k.next().value, k.next().value, k.next().done,
         e.next().value.join(":"), e.next().value.join(":"), e.next().done,
         a[Symbol.iterator] === a.values, v[Symbol.toStringTag]];
r.join();
