// sec-string.prototype.slice: normal "de,bcd,bcd,c,"
// slice counts a negative index from the end; substring (sec-string.prototype.substring)
// clamps it to 0 and swaps its arguments; at (sec-string.prototype.at) counts from the end.
var s = "abcdef";
[s.slice(-3, -1), s.slice(1, 4), s.substring(4, 1), s.at(-4), s.substring(-1, 0)].join();
