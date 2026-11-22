// sec-for-in-and-for-of-statements: normal "ab;3;2;4;5"
// A for-of over a String steps its code points through String.prototype[Symbol.iterator]
// (sec-string.prototype-%symbol.iterator%): a surrogate pair is one value of length 2.
var parts = [], units = 0;
for (const c of "a\u{1F600}b") { parts.push(c); units += c.length; }
var s = new String("hello"), m = 0;
for (let ch of s) m++;
parts[0] + parts[2] + ";" + parts.length + ";" + parts[1].length + ";" + units + ";" + m;
