// sec-array.prototype.indexof: normal "1,-1,3,true,true,false"
// Then sec-array.prototype.lastindexof and sec-array.prototype.includes: indexOf and lastIndexOf skip a hole and miss NaN (IsStrictlyEqual); includes reads a hole as undefined and finds NaN (SameValueZero).
var a = [NaN, 1, , 1];
[a.indexOf(1), a.indexOf(NaN), a.lastIndexOf(1), a.includes(NaN), a.includes(undefined), a.includes(1, -1) === false].join();
