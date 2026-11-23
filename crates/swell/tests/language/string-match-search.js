// sec-string.prototype.match: normal "b|1|3|true|2"
// String.prototype.match and search make a RegExp from a String argument and invoke its method;
// an Object argument is handed to its own method keyed by the symbol.
var found = "abc".match("b")[0];
var index = "a.b".search("\\.");
var digits = "a12b3".match("\\d+").index + "a12b3".match("\\d+")[0].length;
var none = "abc".match("z") === null;
var own = "x".search({ [Symbol.search]: function (s) { return s.length + 1; } });
[found, index, digits, none, own].join("|");
