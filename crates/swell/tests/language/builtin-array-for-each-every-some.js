// sec-array.prototype.foreach: normal "0atrue3,2ctrue3|false|true|true|false"
// forEach calls back for the elements present, ascending, with thisArg; every
// (sec-array.prototype.every) and some (sec-array.prototype.some) answer by ToBoolean of the
// callback's answers; on an empty array every answers true and some false.
var log = [], t = {};
["a", , "c"].forEach(function (v, i, o) { log.push(i + v + (this === t) + o.length); }, t);
var e = [1, 2, 3].every(function (v) { return v < 2; });
var s = [1, 2, 3].some(function (v) { return v > 1; });
[log.join(), e, s, [].every(Boolean), [].some(Boolean)].join("|");
