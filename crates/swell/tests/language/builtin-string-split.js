// sec-string.prototype.split: normal "a/b/c|3|a,b|x;y;z|1|0|,,"
// With a String separator: the parts between its occurrences, at most limit of them; the
// empty separator splits into code units; no separator answers the String alone; a limit of
// 0 answers no part; separators next to each other leave empty parts.
var path = "a b c".split(" ").join("/");
var count = "one\ntwo\nthree".split("\n").length;
var limited = "a,b,c".split(",", 2).join();
var units = "xyz".split("").join(";");
var whole = "a,b".split().length;
var none = "a,b".split(",", 0).length;
var empty = ",,".split(",").join();
[path, count, limited, units, whole, none, empty].join("|");
