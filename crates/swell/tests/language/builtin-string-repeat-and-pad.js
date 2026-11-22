// sec-string.prototype.repeat: normal "abab|ab|  x|x--|1231231abc|abc1231231|abc"
// Then sec-string.prototype.padstart and sec-string.prototype.padend with sec-stringpad: the
// filler repeated and cut to the length asked, a space by default, none when it is empty.
"ab".repeat(2) + "|" + "ab".repeat(1) + "ab".repeat(0) + "|" + "x".padStart(3) + "|" +
  "x".padEnd(3, "-") + "|" + "abc".padStart(10, "123") + "|" + "abc".padEnd(10, "123") + "|" +
  "abc".padEnd(9, "");
