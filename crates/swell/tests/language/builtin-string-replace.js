// sec-string.prototype.replace: normal "a-c-|x[b]y|<b|a|c|$>|A1B|-a-b-|a.b.c"
// Then sec-string.prototype.replaceall, both with a String pattern: the first match or every
// match is replaced, by a String through sec-getsubstitution or by the result of a function
// called with the match and its position; the empty pattern matches before each code unit.
var first = "abcb".replace("b", "-").replace("b", "-");
var marked = "xby".replace("b", "[$&]");
var parts = "abc".replace("b", "<$&|$`|$'|$$>").slice(1, -1);
var upper = "a1b".replace("a", function (m) { return "A"; }).replace("b", function (m, i) { return "B"; });
var dashed = "ab".replaceAll("", "-");
var dotted = "a b c".replaceAll(" ", ".");
[first, marked, parts, upper, dashed, dotted].join("|");
