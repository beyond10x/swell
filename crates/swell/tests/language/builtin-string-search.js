// sec-string.prototype.includes: normal "true,false,true,true,4,1,-1"
// includes, startsWith and endsWith (sec-string.prototype.startswith,
// sec-string.prototype.endswith) search from a position; lastIndexOf
// (sec-string.prototype.lastindexof) searches backwards from it.
var s = "abcabc";
[s.includes("ca"), s.includes("ca", 3), s.startsWith("bc", 1), s.endsWith("ab", 2),
 s.lastIndexOf("bc"), s.lastIndexOf("bc", 3), s.lastIndexOf("x")].join();
