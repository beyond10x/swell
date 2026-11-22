// sec-object.freeze: normal "true,false,false,false,true,false,true,true,false,true,1"
// Object.freeze and Object.seal (sec-object.seal) with SetIntegrityLevel (sec-setintegritylevel), read back by Object.isFrozen and Object.isSealed (sec-testintegritylevel).
var o = Object.freeze({ a: 1 });
var d = Object.getOwnPropertyDescriptor(o, "a");
var a = Object.freeze([1, 2]);
var s = Object.seal({ c: 3 });
var e = Object.getOwnPropertyDescriptor(s, "c");
[Object.isFrozen(o), Object.isExtensible(o), d.writable, d.configurable,
 Object.isFrozen(a), Object.getOwnPropertyDescriptor(a, "length").writable,
 Object.isSealed(s), e.writable, Object.isFrozen(s), Object.isFrozen(1), Object.seal(1)].join();
