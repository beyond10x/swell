// sec-string.prototype.matchall: normal "1@1,22@3|b|1|\x31\.\x2c"
// String.prototype.matchAll hands a global RegExp to RegExp.prototype[Symbol.matchAll], whose
// iterator answers each match with its index and its groups; a RegExp without g answers one
// match from the method itself; RegExp.escape escapes an initial digit, a syntax character and a
// punctuator.
var found = [];
for (var m of "a1b22".matchAll(/\d+/g)) found.push(m[0] + "@" + m.index);
var named = [..."ab".matchAll(/(?<x>b)/g)][0].groups.x;
var once = [.../a/[Symbol.matchAll]("aa")].length;
[found.join(), named, once, RegExp.escape("1.,")].join("|");
